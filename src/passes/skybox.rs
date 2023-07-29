use wgpu::{
    BindGroupDescriptor, BindGroupLayoutDescriptor, BlendState, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, TextureView, VertexState,
};

use crate::{
    cubemap::Cubemap,
    loader::Scene,
    resources::SceneUniform,
    texture::{Sampler, Texture},
    RendererConfig,
};

pub struct Skybox {
    pipeline: wgpu::RenderPipeline,
    cubemap: Cubemap,
    cubemap_sampler: Sampler,
    cubemap_bind_group: wgpu::BindGroup,
}

impl Skybox {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &RendererConfig) -> Self {
        let cubemap = Cubemap::from_dds(device, queue, &config.skybox);
        let cubemap_sampler = Sampler::cubemap_sampler(device);

        let cubemap_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("skybox bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/skybox.wgsl"));

        let cubemap_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("skybox bind group"),
            layout: &cubemap_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cubemap.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&cubemap_sampler.sampler),
                },
            ],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Skybox pipeline layout"),
                bind_group_layouts: &[
                    &SceneUniform::bind_group_layout(device),
                    &cubemap_bind_group_layout,
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
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
                depth_compare: wgpu::CompareFunction::LessEqual,
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

        Skybox {
            cubemap,
            cubemap_sampler,
            pipeline,
            cubemap_bind_group,
        }
    }

    pub fn pass(
        &self,
        scene: &Scene,
        output: &TextureView,
        depth_buffer: &TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Skybox"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_buffer,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &scene.scene.uniform_bind_group, &[]);
            pass.set_bind_group(1, &self.cubemap_bind_group, &[]);
            pass.draw(0..36, 0..1);
        }
    }
}
