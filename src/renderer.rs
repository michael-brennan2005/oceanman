use glam::{vec3, Mat4};
use winit::{
    event::{self, WindowEvent},
    window::Window,
};

use crate::{
    camera::Camera,
    loader,
    resources::{mesh_pipeline, Mesh, MeshUniformData, SceneUniform, SceneUniformData, Texture},
};

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    camera: Camera,
    depth_buffer: Texture,

    scene_uniform: SceneUniform,
    mesh: Mesh,
    mesh_pipeline: wgpu::RenderPipeline,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera = Camera::default();

        let depth_buffer = Texture::create_depth_texture(&device, &config);

        let scene_uniform = SceneUniform::new(&device, SceneUniformData::new_from_camera(&camera));
        let (vertex_buffer, vertex_count) =
            loader::vertex_buffer_from_file(&device, String::from("resources/cube.obj"));
        let mesh = Mesh::new(
            &device,
            vertex_buffer,
            vertex_count,
            MeshUniformData::new(Mat4::from_scale(vec3(0.5, 0.5, 0.5))),
        );

        let mesh_pipeline = mesh_pipeline(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            camera,
            depth_buffer,
            scene_uniform,
            mesh,
            mesh_pipeline,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera.input(event);
    }

    pub fn update(&mut self) {
        self.camera.update();
        self.scene_uniform
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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.mesh_pipeline);
            render_pass.set_bind_group(0, &self.scene_uniform.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.mesh.uniform_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
            render_pass.draw(0..self.mesh.vertex_count, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
