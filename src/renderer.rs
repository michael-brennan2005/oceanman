use std::{fs, path::Path, time::Duration};

use egui::{ClippedPrimitive, Color32, TexturesDelta};
use egui_wgpu::renderer::ScreenDescriptor;
use pollster::block_on;
use wgpu::{RenderPassDescriptor, ShaderModuleDescriptor, TextureUsages};
use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController, FlyingCamera},
    gbuffers::GBuffers,
    loader::Scene,
    passes::{
        self, Compose, Fxaa, FxaaParams, ReloadableShaders, Skybox, Tonemapping, WriteGBuffers,
    },
    resources::SceneUniformData,
    shadowmap::{ShadowData, Shadows},
    texture::Texture,
    RendererConfig,
};

#[derive(Default)]
pub struct RendererUIState {
    shader_error_message: String,
    loader_error_message: String,

    // Debugging shadows
    shadow_theta: f32,
    shadow_phi: f32,
    shadow_dist: f32,

    fxaa_enabled: bool,
    fxaa_params: FxaaParams,
}

pub struct Renderer {
    // wgpu internals
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    // camera
    camera: Camera,
    camera_controller: Box<dyn CameraController>,

    // resources used by multiple passes
    scene: Scene,
    gbuffers: GBuffers,
    shadows: Shadows,
    compose_output: Texture,
    tonemapping_output: Texture,

    // passes
    write_gbuffers: passes::WriteGBuffers,
    compose: passes::Compose,
    skybox: passes::Skybox,
    tonemapping: passes::Tonemapping,
    fxaa: passes::Fxaa,
    egui: egui_wgpu::Renderer,
    egui_state: RendererUIState,
}

impl Renderer {
    pub fn new(window: &Window, renderer_config: &RendererConfig) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device"),
                features: wgpu::Features::TIMESTAMP_QUERY,
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera = Camera::default();
        let camera_controller = Box::new(FlyingCamera::new());
        let scene = match &renderer_config.gltf {
            Some(x) => Scene::from_gltf(&device, &queue, x).unwrap(),
            None => Scene::new(&device),
        };
        let gbuffers = GBuffers::new(&device, &config);
        let shadows = Shadows::new(&device, &config);
        let compose_output = Texture::new(
            &device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba16Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Compose output/Tonemapping input"),
            false,
        );
        let tonemapping_output = Texture::new(
            &device,
            config.width,
            config.height,
            config.format,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Tonemapping out/Fxaa input"),
            false,
        );

        let write_gbuffers = passes::WriteGBuffers::new(&device);
        let compose = passes::Compose::new(&device, &queue, &renderer_config);
        let skybox = passes::Skybox::new(&device, &queue, &renderer_config);
        let tonemapping = passes::Tonemapping::new(&device, &config, &compose_output);
        let fxaa = passes::Fxaa::new(&device, &config, &tonemapping_output);
        let egui = egui_wgpu::renderer::Renderer::new(&device, config.format, None, 1);

        Self {
            surface,
            config,
            device,
            queue,
            camera,
            camera_controller,
            scene,
            gbuffers,
            shadows,
            compose_output,
            write_gbuffers,
            compose,
            skybox,
            tonemapping,
            tonemapping_output,
            egui,
            fxaa,
            egui_state: Default::default(),
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera_controller.input(event);
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera_controller.update(&mut self.camera, dt);
        self.scene
            .scene
            .update(&self.queue, SceneUniformData::new_from_camera(&self.camera));
        self.shadows.update_uniform(
            &self.queue,
            ShadowData::new(
                self.camera.eye,
                self.egui_state.shadow_dist,
                self.egui_state.shadow_theta.to_radians(),
                self.egui_state.shadow_phi.to_radians(),
            ),
        );
        self.fxaa
            .uniform
            .update(&self.queue, self.egui_state.fxaa_params);
    }

    // TODO: seems fragile?
    pub fn reload_shader<T: ReloadableShaders, U: AsRef<Path>>(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        pass: &mut T,
        path: U,
    ) -> Result<(), String> {
        let path = Path::new("src/").join(path.as_ref().strip_prefix("../").unwrap());
        let Ok(code) = fs::read(&path) else { return Err(String::from("Error reading file.")); };

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let new_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(String::from_utf8_lossy(code.as_slice())),
            debug: true,
        });
        let result = device.pop_error_scope();
        match block_on(result) {
            Some(err) => Err(err.to_string()),
            None => {
                pass.reload(device, config, new_shader);
                Ok(())
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Renderer").show(ctx, |ui| {
            self.camera_controller.ui(&mut self.camera, ui);
            // omfg are you fr
            macro_rules! shaders_helper {
                ($ui:ident, $lowercase:ident, $uppercase:ident) => {
                    $ui.label(egui::RichText::new(stringify!($uppercase)).strong());
                    $ui.end_row();

                    let available_shaders: &[&str] = $uppercase::available_shaders();

                    for i in available_shaders.iter().enumerate() {
                        $ui.label(*i.1);
                        if $ui.button("Reload").clicked() {
                            let error_message = Renderer::reload_shader(
                                &self.device,
                                &self.config,
                                &mut self.$lowercase,
                                *i.1,
                            );
                            self.egui_state.shader_error_message = match error_message {
                                Ok(_) => String::from(""),
                                Err(x) => x,
                            }
                        }
                        $ui.end_row();
                    }
                };
            }
            egui::CollapsingHeader::new("Shaders").show(ui, |ui| {
                egui::Grid::new("shaders").show(ui, |ui| {
                    shaders_helper!(ui, write_gbuffers, WriteGBuffers);
                    shaders_helper!(ui, compose, Compose);
                    shaders_helper!(ui, skybox, Skybox);
                    shaders_helper!(ui, tonemapping, Tonemapping);
                    shaders_helper!(ui, fxaa, Fxaa);
                });

                ui.label(
                    egui::RichText::new(&self.egui_state.shader_error_message).color(Color32::RED),
                );
            });

            egui::CollapsingHeader::new("Loader").show(ui, |ui| {
                if ui.button("Load glTF").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("glTF", &["gltf", "glb"])
                        .pick_file()
                    {
                        let scene = Scene::from_gltf(
                            &self.device,
                            &self.queue,
                            &String::from(path.to_str().unwrap()),
                        );
                        match scene {
                            Ok(_) => self.scene = scene.unwrap(),
                            Err(_) => {
                                self.egui_state.loader_error_message =
                                    String::from("Failed to load glTF.")
                            }
                        }
                    }
                }

                if ui.button("Load skybox").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("dds", &["dds"])
                        .pick_file()
                    {
                        self.skybox.update_cubemap(
                            &self.device,
                            &self.queue,
                            &String::from(path.to_str().unwrap()),
                        );
                    }
                }

                if ui.button("Load irradiance (diffuse)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("dds", &["dds"])
                        .pick_file()
                    {
                        self.compose.ibl.update(
                            &self.device,
                            &self.queue,
                            Some(&String::from(path.to_str().unwrap())),
                            None,
                        );
                    }
                }

                if ui.button("Load prefilter (specular)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("dds", &["dds"])
                        .pick_file()
                    {
                        self.compose.ibl.update(
                            &self.device,
                            &self.queue,
                            None,
                            Some(&String::from(path.to_str().unwrap())),
                        );
                    }
                }
            });

            egui::CollapsingHeader::new("Shadows").show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.egui_state.shadow_theta, 0.0..=360.0)
                        .text("Theta")
                        .show_value(true),
                );

                ui.add(
                    egui::Slider::new(&mut self.egui_state.shadow_phi, 0.0..=360.0)
                        .text("Phi")
                        .show_value(true),
                );

                ui.add(
                    egui::Slider::new(&mut self.egui_state.shadow_dist, 0.0..=360.0)
                        .text("Distance")
                        .show_value(true),
                );
            });

            egui::CollapsingHeader::new("FXAA").show(ui, |ui| {
                ui.checkbox(&mut self.egui_state.fxaa_enabled, "FXAA");
                ui.add(
                    egui::Slider::new(&mut self.egui_state.fxaa_params.edge_threshold, 0.0..=1.0)
                        .step_by(1.0 / 16.0)
                        .text("Edge threshold"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.egui_state.fxaa_params.edge_threshold_min,
                        0.0..=1.0,
                    )
                    .step_by(1.0 / 32.0)
                    .text("Edge threshold min"),
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.egui_state.fxaa_params.search_acceleration,
                        0.0..=4.0,
                    )
                    .step_by(1.0)
                    .text("Search acceleration"),
                );
            })
        });
    }

    pub fn render(
        &mut self,
        egui_textures_delta: &TexturesDelta,
        egui_clipped_primitves: &Vec<ClippedPrimitive>,
        egui_screen_descriptor: &ScreenDescriptor,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command encoder"),
            });

        self.write_gbuffers
            .pass(&self.scene, &self.gbuffers, &mut encoder);
        self.compose.pass(
            &self.scene,
            &self.gbuffers,
            &self.compose_output.view,
            &mut encoder,
        );
        self.skybox.pass(
            &self.scene,
            &self.compose_output.view,
            &self.gbuffers.depth.view,
            &mut encoder,
        );

        if self.egui_state.fxaa_enabled {
            self.tonemapping
                .pass(&self.tonemapping_output.view, &mut encoder);
            self.fxaa.pass(&view, &mut encoder);
        } else {
            self.tonemapping.pass(&view, &mut encoder);
        }

        // TODO: put into its own pass/make nicer
        for delta in &egui_textures_delta.set {
            self.egui
                .update_texture(&self.device, &self.queue, delta.0, &delta.1);
        }
        self.egui.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &egui_clipped_primitves,
            egui_screen_descriptor,
        );
        {
            let mut egui_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("UI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui.render(
                &mut egui_pass,
                &egui_clipped_primitves,
                egui_screen_descriptor,
            );
        }
        for delta in &egui_textures_delta.free {
            self.egui.free_texture(delta);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();
        Ok(())
    }
}
