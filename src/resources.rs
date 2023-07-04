use std::num::NonZeroU64;

use glam::{vec3, vec4, Mat4, UVec3, Vec3, Vec4};
use wgpu::{
    util::DeviceExt, BlendState, BufferBinding, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, RenderPipeline, ShaderStages, VertexBufferLayout,
    VertexState,
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

    pub fn shadow(orthographic_projection_size: Vec3, lighting_direction: Vec3) -> Self {
        // to make the multiplication read easier
        let ops = orthographic_projection_size;
        let perspective_view = Mat4::orthographic_lh(-ops.x, ops.x, -ops.y, ops.y, -ops.z, ops.z)
            * Mat4::look_to_lh(vec3(0.0, 0.0, 0.0), lighting_direction, vec3(0.0, 1.0, 0.0));

        Self {
            perspective_view,
            camera_position: vec4(0.0, 0.0, 0.0, 1.0),
        }
    }
}

/// SceneUniform has two SceneUniformData's.
/// SceneUniformData at slot 0 is for scene pass (rendering to camera),
/// SceneUniformData at slot 1 is for shadow pass (rendering to shadowmap)
pub struct SceneUniform {
    pub uniform_buffer_0: wgpu::Buffer,
    pub uniform_buffer_1: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

// TODO: write out the min_binding_sizes (avoid checks at draw call)
impl SceneUniform {
    pub fn new(device: &wgpu::Device, zero: SceneUniformData, one: SceneUniformData) -> Self {
        let uniform_buffer_0 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene uniform buffer - scene pass uniform"),
            contents: bytemuck::cast_slice(&[zero]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniform_buffer_1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene uniform buffer - shadow pass uniform"),
            contents: bytemuck::cast_slice(&[one]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene uniform bind group"),
            layout: &SceneUniform::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer_0.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        uniform_buffer_1.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        SceneUniform {
            uniform_buffer_0,
            uniform_buffer_1,
            uniform_bind_group,
        }
    }

    // TODO: cover case in which we need to update shadow pass
    pub fn update(&self, queue: &wgpu::Queue, data: SceneUniformData) {
        queue.write_buffer(&self.uniform_buffer_0, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<SceneUniformData>() as u64
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<SceneUniformData>() as u64
                        ),
                    },
                    count: None,
                },
            ],
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LightingUniformData {
    pub direction: Vec4,
    pub color: Vec4,
}

pub struct LightingUniform {
    pub uniform_buffer: wgpu::Buffer,
    pub shadow_map: Texture,
    pub uniform_bind_group: wgpu::BindGroup,
}

unsafe impl bytemuck::Pod for LightingUniformData {}
unsafe impl bytemuck::Zeroable for LightingUniformData {}

// TODO: write out the min_binding_sizes (avoid checks at draw call)
// Contains shadow map (since this is used only for scene pass and not shadow pass)
// FIXME: seems like a bad idea to put shadowmap in here?
impl LightingUniform {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        data: LightingUniformData,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lighting uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let shadow_map = Texture::create_depth_texture(&device, &config);

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lighting uniform bind group"),
            layout: &LightingUniform::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                },
            ],
        });

        LightingUniform {
            uniform_buffer,
            shadow_map,
            uniform_bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: LightingUniformData) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Lighting uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        })
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct MaterialUniformData {
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
}

unsafe impl bytemuck::Pod for MaterialUniformData {}
unsafe impl bytemuck::Zeroable for MaterialUniformData {}

impl Default for MaterialUniformData {
    fn default() -> Self {
        MaterialUniformData {
            ambient: Vec4::ONE,
            diffuse: Vec4::ONE,
            specular: Vec4::ONE,
        }
    }
}

impl MaterialUniformData {
    pub fn new(ambient: Vec3, diffuse: Vec3, specular: Vec3) -> Self {
        MaterialUniformData {
            ambient: (ambient, 1.0).into(),
            diffuse: (diffuse, 1.0).into(),
            specular: (specular, 1.0).into(),
        }
    }
}
pub struct Material {
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub diffuse_texture: Texture,
}

impl Material {
    pub fn new(device: &wgpu::Device, data: MaterialUniformData, diffuse_texture: Texture) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material bind group"),
            layout: &Material::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
            ],
        });

        Self {
            uniform_buffer,
            bind_group,
            diffuse_texture,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshUniformData {
    pub world: Mat4,
}

unsafe impl bytemuck::Pod for MeshUniformData {}
unsafe impl bytemuck::Zeroable for MeshUniformData {}

impl MeshUniformData {
    pub fn new(world: Mat4) -> Self {
        MeshUniformData {
            world: world.into(),
        }
    }
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub material_index: usize,
}

// TODO: write out the min_binding_sizes (avoid checks at draw call)
impl Mesh {
    pub fn new(
        device: &wgpu::Device,
        vertex_buffer: wgpu::Buffer,
        vertex_count: u32,
        data: MeshUniformData,
        material_index: usize,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            bind_group,
            material_index,
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

    pub fn create_from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Self {
        let image = image::load_from_memory(bytes).unwrap().to_rgba8();
        let dimensions = image.dimensions();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            ..Default::default()
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn create_1x1_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: [u8; 4],
        label: Option<&str>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            ..Default::default()
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        Self {
            texture,
            view,
            sampler,
        }
    }

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
            compare: Some(wgpu::CompareFunction::Less),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            anisotropy_clamp: 1,
            border_color: None,
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

pub fn shadow_pipeline(
    device: &Device,
    surface_config: &wgpu::SurfaceConfiguration,
) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shadow_pipeline.wgsl"));

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shadow pipeline"),
        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Shadow pipeline layout"),
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
            targets: &[],
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
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
                &LightingUniform::bind_group_layout(device),
                &Material::bind_group_layout(device),
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
            cull_mode: None,
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
