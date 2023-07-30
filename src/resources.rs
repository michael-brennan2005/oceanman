use std::num::NonZeroU64;

use glam::{vec3, vec4, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use crate::{
    camera::Camera,
    texture::{Sampler, Texture},
};

#[macro_use]
macro_rules! bytemuck_impl {
    ($struct_name:ident) => {
        unsafe impl bytemuck::Pod for $struct_name {}
        unsafe impl bytemuck::Zeroable for $struct_name {}
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SceneUniformData {
    pub perspective: Mat4,
    pub view: Mat4,
    pub inverse_perspective_view: Mat4,
    pub camera_position: Vec4,
}
bytemuck_impl!(SceneUniformData);

impl SceneUniformData {
    pub fn new() -> Self {
        Self {
            perspective: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            inverse_perspective_view: Mat4::IDENTITY.inverse(),
            camera_position: vec4(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn new_from_camera(camera: &Camera) -> Self {
        let perspective = Mat4::perspective_lh(45.0_f32.to_radians(), 1600.0 / 900.0, 0.01, 100.0);
        let (view, camera_position) = camera.build_uniforms();
        Self {
            perspective,
            view,
            inverse_perspective_view: (perspective * view).inverse(),
            camera_position,
        }
    }

    pub fn shadow(orthographic_projection_size: Vec3, lighting_direction: Vec3) -> Self {
        // to make the multiplication read easier
        let ops = orthographic_projection_size;
        let perspective_view = Mat4::orthographic_lh(-ops.x, ops.x, -ops.y, ops.y, -ops.z, ops.z)
            * Mat4::look_to_lh(vec3(0.0, 0.0, 0.0), lighting_direction, vec3(0.0, 1.0, 0.0));

        // FIXME: super broken
        Self {
            perspective: perspective_view,
            view: perspective_view,
            inverse_perspective_view: perspective_view.inverse(),
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

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct LightingUniformData {
    pub count: u32,
    pub colors: [Vec4; 16],
    pub positions: [Vec4; 16],
}
bytemuck_impl!(LightingUniformData);

impl LightingUniformData {
    // 0th element is position, 1st element is color
    pub fn new(lighting: Vec<(Vec3, Vec3)>) -> Self {
        let count = lighting.len().min(16) as u32;
        let mut colors: [Vec4; 16] = [vec4(0.0, 0.0, 0.0, 0.0); 16];
        let mut positions: [Vec4; 16] = [vec4(0.0, 0.0, 0.0, 0.0); 16];

        for i in 0..(count as usize) {
            positions[i] = (lighting[i].0, 1.0).into();
            colors[i] = (lighting[i].1, 1.0).into();
        }

        Self {
            colors,
            positions,
            count,
        }
    }
}

pub struct LightingUniform {
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl LightingUniform {
    pub fn new(device: &wgpu::Device, data: LightingUniformData) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lighting uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Lighting uniform bind group"),
            layout: &LightingUniform::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        LightingUniform {
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: LightingUniformData) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Lighting uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        std::mem::size_of::<LightingUniformData>() as u64
                    ),
                },
                count: None,
            }],
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
bytemuck_impl!(MaterialUniformData);

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
    pub diffuse_texture_sampler: Sampler,
    pub normal_texture: Texture,
    pub normal_texture_sampler: Sampler,
    pub metal_roughness_texture: Texture,
    pub metal_roughness_texture_sampler: Sampler,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        data: MaterialUniformData,
        diffuse_texture: Texture,
        normal_texture: Texture,
        metal_roughness_texture: Texture,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material uniform buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let diffuse_texture_sampler = Sampler::diffuse_texture_sampler(&device);
        let normal_texture_sampler = Sampler::normal_texture_sampler(&device);
        let metal_roughness_texture_sampler = Sampler::normal_texture_sampler(&device);

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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture_sampler.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&normal_texture_sampler.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&metal_roughness_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(
                        &metal_roughness_texture_sampler.sampler,
                    ),
                },
            ],
        });

        Self {
            uniform_buffer,
            bind_group,
            diffuse_texture,
            diffuse_texture_sampler,
            normal_texture,
            normal_texture_sampler,
            metal_roughness_texture,
            metal_roughness_texture_sampler,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
bytemuck_impl!(MeshUniformData);

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
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub material_index: usize,
}

impl Mesh {
    pub fn new(
        device: &wgpu::Device,
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        vertex_count: u32,
        index_count: u32,
        data: MeshUniformData,
        material_index: usize,
        name: Option<&str>,
    ) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("Uniform buffer for {}", name.unwrap_or("Mesh")).as_str()),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("Bind group {}", name.unwrap_or("Mesh")).as_str()),
            layout: &Mesh::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            vertex_buffer,
            vertex_count,
            index_buffer,
            index_count,
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
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<MeshUniformData>() as u64),
                },
                count: None,
            }],
        })
    }
}
