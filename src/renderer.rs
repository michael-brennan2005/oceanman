use std::time::Duration;

use wgpu::{BufferUsages, QuerySetDescriptor, TextureUsages};
use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController, FlyingCamera},
    gbuffers::GBuffers,
    loader::Scene,
    passes,
    resources::SceneUniformData,
    texture::Texture,
    RendererConfig,
};

pub struct TimestampQueries {
    timestamps: wgpu::QuerySet,
    timestamp_period: f32,
    data_buffer: wgpu::Buffer,
    mapping_buffer: wgpu::Buffer,
}

impl TimestampQueries {
    const NUM_PASSES: u32 = 4;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let timestamps = device.create_query_set(&QuerySetDescriptor {
            label: Some("Timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2_u32 * Self::NUM_PASSES,
        });

        let timestamp_period = queue.get_timestamp_period();

        let data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Timestamps data buffer"),
            size: (2_u32 * Self::NUM_PASSES * std::mem::size_of::<u64>() as u32) as u64,
            usage: BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE,
            mapped_at_creation: false,
        });

        let mapping_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Timestamps mapping buffer"),
            size: (2_u32 * Self::NUM_PASSES * std::mem::size_of::<u64>() as u32) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            timestamps,
            timestamp_period,
            data_buffer,
            mapping_buffer,
        }
    }
}

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    timestamps: TimestampQueries,

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
    pub async fn new(window: &Window, renderer_config: &RendererConfig) -> Self {
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
                    features: wgpu::Features::TIMESTAMP_QUERY,
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

        let timestamps = TimestampQueries::new(&device, &queue);

        let camera = Camera::default();
        let camera_controller = Box::new(FlyingCamera::new());
        let scene = Scene::from_gltf(&device, &queue, &renderer_config.scene).unwrap();

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
        let compose = passes::Compose::new(&device, &queue, &renderer_config);
        let skybox = passes::Skybox::new(&device, &queue, &renderer_config);
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
            timestamps,
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

        encoder.write_timestamp(&self.timestamps.timestamps, 0);
        self.write_gbuffers
            .pass(&self.scene, &self.gbuffers, &mut encoder);
        encoder.write_timestamp(&self.timestamps.timestamps, 1);
        encoder.write_timestamp(&self.timestamps.timestamps, 2);
        self.compose.pass(
            &self.scene,
            &self.gbuffers,
            &self.compose_output.view,
            &mut encoder,
        );
        encoder.write_timestamp(&self.timestamps.timestamps, 3);
        encoder.write_timestamp(&self.timestamps.timestamps, 4);
        self.skybox.pass(
            &self.scene,
            &self.compose_output.view,
            &self.gbuffers.depth.view,
            &mut encoder,
        );
        encoder.write_timestamp(&self.timestamps.timestamps, 5);
        encoder.write_timestamp(&self.timestamps.timestamps, 6);
        self.tonemapping.pass(&view, &mut encoder);
        encoder.write_timestamp(&self.timestamps.timestamps, 7);

        encoder.resolve_query_set(
            &self.timestamps.timestamps,
            0..8,
            &self.timestamps.data_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamps.data_buffer,
            0,
            &self.timestamps.mapping_buffer,
            0,
            (2 * TimestampQueries::NUM_PASSES * std::mem::size_of::<u64>() as u32) as u64,
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        self.timestamps
            .mapping_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::MaintainBase::Wait);
        {
            let binding = &self.timestamps.mapping_buffer.slice(..).get_mapped_range();
            let timestamp_data: &[u64] = bytemuck::cast_slice(&binding);

            for times in timestamp_data.chunks(2).enumerate() {
                let duration_ns =
                    (times.1[1] - times.1[0]) as f32 * self.timestamps.timestamp_period;
                let pass = match times.0 {
                    0 => "Write GBuffers",
                    1 => "Compose",
                    2 => "Skybox",
                    3 => "Tonemapping",
                    _ => "UNKNOWN",
                };
                println!("{} took {}us to complete.", pass, duration_ns / 1000.0);
            }
            println!("-------------------------");
        }
        self.timestamps.mapping_buffer.unmap();
        output.present();
        Ok(())
    }
}
