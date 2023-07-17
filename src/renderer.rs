use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::Camera,
    gbuffers::GBuffers,
    loader::Scene,
    passes,
    pipelines::{mesh_pipeline, shadow_pipeline},
    resources::SceneUniformData,
    texture::Texture,
};

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    camera: Camera,
    scene: Scene,
    gbuffers: GBuffers,

    compose_output: Texture,

    write_gbuffers: passes::WriteGBuffers,
    compose: passes::Compose,
    tonemapping: passes::Tonemapping,
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
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
        let scene = Scene::from_gltf(
            &device,
            &config,
            &queue,
            "resources/glTF-Sample-Models/2.0/MetalRoughSpheres/glTF/MetalRoughSpheres.gltf"
                .to_string(),
        )
        .unwrap();

        let gbuffers = GBuffers::new(&device, &config);
        let compose_output = Texture::create(
            &device,
            config.width,
            config.height,
            Some("Compose output/Tonemapping input"),
            Some(wgpu::TextureFormat::Rgba16Float),
        );

        let write_gbuffers = passes::WriteGBuffers::new(&device, &config);
        let compose = passes::Compose::new(&device);
        let tonemapping = passes::Tonemapping::new(&device, &config, &compose_output);
        Self {
            surface,
            device,
            queue,
            config,
            size,
            camera,
            scene,
            gbuffers,
            compose_output,
            write_gbuffers,
            compose,
            tonemapping,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera.input(event);
    }

    pub fn update(&mut self) {
        self.camera.update();
        self.scene
            .scene
            .update(&self.queue, SceneUniformData::new_from_camera(&self.camera));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command encoder"),
            });

        //        {
        //            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //                label: Some("Shadow pass"),
        //                color_attachments: &[],
        //                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        //                    view: &self.scene.lighting.shadow_map.view,
        //                    depth_ops: Some(wgpu::Operations {
        //                        load: wgpu::LoadOp::Clear(1.0),
        //                        store: true,
        //                    }),
        //                    stencil_ops: None,
        //                }),
        //            });
        //
        //            shadow_pass.set_pipeline(&self.shadow_pipeline);
        //            shadow_pass.set_bind_group(0, &self.scene.scene.uniform_bind_group, &[]);
        //
        //            for mesh in &self.scene.meshes {
        //                shadow_pass.set_bind_group(1, &mesh.bind_group, &[]);
        //
        //                shadow_pass
        //                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        //                shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        //                shadow_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        //            }
        //        }

        self.write_gbuffers
            .pass(&self.scene, &self.gbuffers, &mut encoder);
        self.compose.pass(
            &self.scene,
            &self.gbuffers,
            &self.compose_output.view,
            &mut encoder,
        );

        self.tonemapping.pass(&view, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
