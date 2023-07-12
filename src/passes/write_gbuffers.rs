use std::{rc::Rc, sync::Arc};

use wgpu::{
    BlendState, FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipeline, VertexState,
};

use crate::{
    common::VertexAttributes,
    loader::Scene,
    resources::{Material, Mesh, SceneUniform},
    texture::Texture,
};

// TODO: This is a resource, put bind group stuff in here
pub struct GBuffers {
    /// World position of fragment, RGBA32
    pub position: Texture,
    /// Albedo of fragment, RGBA8
    pub albedo: Texture,
    /// Normal of fragment (world space), RGBA8
    pub normal: Texture,
    /// Material of fragment, RGBA8 (sort of TODO at moment)
    pub material: Texture,
}

impl GBuffers {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let position = Texture::create(
            device,
            config.width,
            config.height,
            Some("Gbuffers - position (worldspace)"),
            Some(wgpu::TextureFormat::Rgba32Float),
        );

        // TODO: color tonemapping issues. get a decal image (like a logo or some random thing) and see what it does.
        let albedo = Texture::create(
            device,
            config.width,
            config.height,
            Some("Gbuffers - albedo"),
            Some(wgpu::TextureFormat::Rgba8Unorm),
        );

        let normal = Texture::create(
            device,
            config.width,
            config.height,
            Some("Gbuffers - normal (worldspace)"),
            Some(wgpu::TextureFormat::Rgba8Unorm),
        );

        let material = Texture::create(
            device,
            config.width,
            config.height,
            Some("Gbuffers - material"),
            Some(wgpu::TextureFormat::Rgba8Unorm),
        );

        Self {
            position,
            albedo,
            normal,
            material,
        }
    }
}

pub struct WriteGBuffers {
    /// FIXME: I think this needs to be RC because it will be used by Compose pass, but i am not sure.
    pub gbuffers: Rc<GBuffers>,
    pipeline: wgpu::RenderPipeline,
    depth_buffer: Texture,
}

impl WriteGBuffers {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/write_gbuffers.wgsl"));
        let gbuffers = Rc::new(GBuffers::new(device, config));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
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
        });

        let depth_buffer = Texture::create_depth_texture(device, config);
        Self {
            gbuffers,
            pipeline,
            depth_buffer,
        }
    }

    /// Complete a GBuffersPass. Pass in the encoder that is being used for the whole render graph.
    pub fn pass(&self, scene: &Scene, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Write GBuffers"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffers.position.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffers.albedo.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffers.normal.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffers.material.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer.view,
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
