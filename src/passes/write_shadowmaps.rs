use std::num::NonZeroU64;

use glam::{vec3, Mat4, Vec3};
use wgpu::{
    include_wgsl, Device, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPipeline, ShaderModule, VertexState,
};

use crate::{
    bytemuck_impl,
    common::VertexAttributes,
    loader::Scene,
    resources::Mesh,
    shadowmap::{ShadowData, Shadowmap, Shadows},
    texture::Texture,
    uniform::Uniform,
};

pub struct WriteShadowmaps {
    pipeline: wgpu::RenderPipeline,
}

impl WriteShadowmaps {
    pub fn new(device: &Device, queue: &Queue, data: ShadowData) -> Self {
        let shader =
            device.create_shader_module(include_wgsl!("../shaders/write_shadowmaps.wgsl", true));

        let pipeline = WriteShadowmaps::pipeline(device, &shader);

        Self { pipeline }
    }

    pub fn pipeline(device: &wgpu::Device, shader: &ShaderModule) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Write shadowmaps pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Write shadowmaps pipeline layout"),
                bind_group_layouts: &[
                    &Uniform::<ShadowData>::bind_group_layout(device),
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

    pub fn pass(&self, scene: &Scene, shadows: &Shadows, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Write Shadowmaps pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadows.shadowmap.texture.view,
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
            pass.set_bind_group(0, &shadows.uniform.bind_group, &[]);

            for mesh in &scene.meshes {
                pass.set_bind_group(1, &mesh.bind_group, &[]);
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
    }
}
