use glam::{vec4, Mat4, Vec3, Vec4};
use wgpu::{
    util::DeviceExt, BlendState, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, VertexBufferLayout, VertexState,
};

use crate::camera::Camera;

// TODO: use perspective_view instead of perspective * view, and then have a camera struct build it for us.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SceneUniformData {
    pub perspective_view: Mat4,
    pub camera_position: Vec4,
}
unsafe impl bytemuck::Pod for SceneUniformData {}
unsafe impl bytemuck::Zeroable for SceneUniformData {}

impl SceneUniformData {
    pub fn new() -> Self {
        Self {
            perspective_view: Mat4::IDENTITY,
            camera_position: vec4(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn new_from_camera(camera: &Camera) -> Self {
        let (perspective_view, camera_position) = camera.build_uniforms();
        Self {
            perspective_view,
            camera_position,
        }
    }
}

pub struct SceneUniform {
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

// TODO: write out the min_binding_sizes (avoid checks at draw call)
impl SceneUniform {
    pub fn new(device: &wgpu::Device, data: SceneUniformData) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene uniform bind group"),
            layout: &SceneUniform::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        SceneUniform {
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: SceneUniformData) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshUniformData {
    pub world: Mat4,
    pub normal: Mat4,
}
unsafe impl bytemuck::Pod for MeshUniformData {}
unsafe impl bytemuck::Zeroable for MeshUniformData {}

impl MeshUniformData {
    pub fn new(world: Mat4) -> Self {
        MeshUniformData {
            world: world.into(),
            normal: world.transpose().inverse(),
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

// TODO: write out the min_binding_sizes (avoid checks at draw call)
impl Mesh {
    pub fn new(
        device: &wgpu::Device,
        vertex_buffer: wgpu::Buffer,
        vertex_count: u32,
        data: MeshUniformData,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh uniform bind group"),
            layout: &Mesh::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        Self {
            vertex_buffer,
            vertex_count,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: MeshUniformData) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Depth texture sampler (unused)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

fn vertex_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: 8 * 4,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 3 * 4,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 6 * 4,
                shader_location: 2,
            },
        ],
    }
}

pub fn mesh_pipeline(
    device: &Device,
    surface_config: &wgpu::SurfaceConfiguration,
) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mesh_pipeline.wgsl"));

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh pipeline"),
        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &[
                &SceneUniform::bind_group_layout(device),
                &Mesh::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[vertex_buffer_layout()],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },

        multiview: None,
    })
}
