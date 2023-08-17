use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages,
};

pub struct Uniform<T: Pod + Zeroable> {
    data: T,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl<T: Pod + Zeroable> Uniform<T> {
    pub fn new(device: &wgpu::Device, label: Option<&str>, data: T) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("Uniform buffer - {}", label.unwrap_or("")).as_str()),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(format!("Bind group - {}", label.unwrap_or("")).as_str()),
            layout: &Uniform::<T>::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            bind_group,
            data,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, data: T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Uniform bind group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<T>() as u64),
                },
                count: None,
            }],
        })
    }

    pub fn bind_group_layout_entry(i: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: i,
            visibility: ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(std::mem::size_of::<T>() as u64),
            },
            count: None,
        }
    }

    pub fn bind_group_entry(&self, i: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: i,
            resource: self.buffer.as_entire_binding(),
        }
    }
}
