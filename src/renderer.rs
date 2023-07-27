use std::time::Duration;

use wgpu::TextureUsages;
use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController, FlyingCamera},
    gbuffers::GBuffers,
    loader::Scene,
    passes,
    resources::SceneUniformData,
    texture::Texture,
};

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera: Camera,
    camera_controller: Box<dyn CameraController>,

    scene: Scene,
    gbuffers: GBuffers,

    compose_output: Texture,

    write_gbuffers: passes::WriteGBuffers,
    compose: passes::Compose,
    skybox: passes::Skybox,
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
        let camera_controller = Box::new(FlyingCamera::new());
        let scene = Scene::from_gltf(
            &device,
            &queue,
            "resources/glTF-Sample-Models/2.0/WaterBottle/glTF/WaterBottle.gltf".to_string(),
        )
        .unwrap();

        let gbuffers = GBuffers::new(&device, &config);
        let compose_output = Texture::new(
            &device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba16Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Compose output/Tonemapping input"),
        );

        let write_gbuffers = passes::WriteGBuffers::new(&device);
        let compose = passes::Compose::new(&device, &queue);
        let skybox = passes::Skybox::new(&device, &queue);
        let tonemapping = passes::Tonemapping::new(&device, &config, &compose_output);

        Self {
            surface,
            device,
            queue,
            camera,
            camera_controller,
            scene,
            gbuffers,
            compose_output,
            write_gbuffers,
            compose,
            skybox,
            tonemapping,
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
        self.tonemapping.pass(&view, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
