use std::num::NonZeroU64;

use glam::{vec3, Mat4, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferUsages, Device, Queue, ShaderStages,
};

use crate::{bytemuck_impl, scene_resources::SceneUniformData};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShadowUniformData {
    pub projection: Mat4,
}
bytemuck_impl!(ShadowUniformData);

impl ShadowUniformData {
    /// pos should be camera pos, theta and phi are where direcitonal light originates from,
    /// looking from center of unit sphere.
    pub fn new(pos: Vec3, theta: f32, phi: f32) -> Self {
        let dir = -vec3(
            f32::sin(phi) * f32::cos(theta),
            f32::sin(phi) * f32::sin(theta),
            f32::cos(phi),
        );

        let right = vec3(f32::sin(theta), f32::cos(theta), 0.0);
        let up = right.cross(dir);

        let eye = pos + (-dir * 50.0);

        let view = Mat4::look_to_lh(eye, dir, up);
        let perspecitve = Mat4::orthographic_lh(-25.0, 25.0, -25.0, 25.0, 0.0, 100.0);
        Self {
            projection: perspecitve * view,
        }
    }
}

pub struct ShadowUniform {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl ShadowUniform {
    pub fn new(device: &Device, queue: &Queue, data: ShadowUniformData) -> Self {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Shadow uniform"),
            contents: bytemuck::cast_slice(&[data]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shadow uniform bind group"),
            layout: &ShadowUniform::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self { buffer, bind_group }
    }

    pub fn bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        std::mem::size_of::<ShadowUniformData>() as u64
                    ),
                },
                count: None,
            }],
        })
    }
}

pub struct WriteShadowmaps {
    pipeline: wgpu::RenderPipeline,
    shadow_uniform: ShadowUniform,
}
