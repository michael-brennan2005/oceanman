use wgpu::{
    BlendState, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipeline, ShaderModule, VertexState,
};

use crate::{
    common::VertexAttributes,
    gbuffers::GBuffers,
    loader::Scene,
    resources::{Material, Mesh, SceneUniform},
    texture::Texture,
};

use super::ReloadableShaders;

pub struct WriteGBuffers {
    pipeline: wgpu::RenderPipeline,
}

impl WriteGBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/write_gbuffers.wgsl"));

        let pipeline = WriteGBuffers::pipeline(device, &shader);
        Self { pipeline }
    }

    pub fn pipeline(device: &wgpu::Device, shader: &ShaderModule) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Write gbuffers pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Write gbuffers pipeline layout"),
                bind_group_layouts: &[
                    &SceneUniform::bind_group_layout(device),
                    &Material::bind_group_layout(device),
                    &Mesh::bind_group_layout(device),
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexAttributes::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: Some(BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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

    /// Complete a GBuffersPass. Pass in the encoder that is being used for the whole render graph.
    pub fn pass(&self, scene: &Scene, gbuffers: &GBuffers, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Write GBuffers"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffers.albedo.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffers.normal.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &gbuffers.material.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gbuffers.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &scene.scene.uniform_bind_group, &[]);

            for mesh in &scene.meshes {
                pass.set_bind_group(1, &scene.materials[mesh.material_index].bind_group, &[]);
                pass.set_bind_group(2, &mesh.bind_group, &[]);
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
    }
}

impl ReloadableShaders for WriteGBuffers {
    fn available_shaders() -> &'static [&'static str] {
        &["../shaders/write_gbuffers.wgsl"]
    }

    fn reload(
        &mut self,
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        index: usize,
        shader_module: wgpu::ShaderModule,
    ) {
        self.pipeline = WriteGBuffers::pipeline(device, &shader_module);
    }
}
