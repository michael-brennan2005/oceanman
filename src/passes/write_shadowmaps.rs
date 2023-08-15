use std::num::NonZeroU64;

use glam::{vec3, Mat4, Vec3};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferUsages, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline, ShaderModule, ShaderStages,
    VertexState,
};

use crate::{
    bytemuck_impl,
    common::VertexAttributes,
    loader::Scene,
    resources::{Mesh, SceneUniformData},
    shadowmap::Shadowmap,
    texture::Texture,
};

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

    pub fn update(&self, queue: &Queue, data: ShadowUniformData) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
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

impl WriteShadowmaps {
    pub fn new(device: &Device, queue: &Queue, data: ShadowUniformData) -> Self {
        let shader =
            device.create_shader_module(include_wgsl!("../shaders/write_shadowmaps.wgsl", true));

        let shadow_uniform = ShadowUniform::new(device, queue, data);
        let pipeline = WriteShadowmaps::pipeline(device, &shader);

        Self {
            pipeline,
            shadow_uniform,
        }
    }

    pub fn update_shadow_uniform(&self, queue: &Queue, data: ShadowUniformData) {
        self.shadow_uniform.update(queue, data);
    }

    pub fn pipeline(device: &wgpu::Device, shader: &ShaderModule) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Write shadowmaps pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Write shadowmaps pipeline layout"),
                bind_group_layouts: &[
                    &ShadowUniform::bind_group_layout(device),
                    &Mesh::bind_group_layout(device),
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexAttributes::buffer_layout()],
            },
            fragment: None,
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

    pub fn pass(&self, scene: &Scene, shadowmap: &Shadowmap, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Write Shadowmaps pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadowmap.texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.shadow_uniform.bind_group, &[]);

            for mesh in &scene.meshes {
                pass.set_bind_group(1, &mesh.bind_group, &[]);
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
    }
}
