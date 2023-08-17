use wgpu::{
    BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BlendState, Device,
    FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
    ShaderModule, TextureView, VertexState,
};

use crate::{
    cubemap::Cubemap,
    loader::Scene,
    resources::SceneUniform,
    texture::{Sampler, Texture},
    RendererConfig,
};

use super::ReloadableShaders;

pub struct Skybox {
    pipeline: wgpu::RenderPipeline,
    cubemap: Cubemap,
    cubemap_sampler: Sampler,
    cubemap_bind_group: wgpu::BindGroup,
}

impl Skybox {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &RendererConfig) -> Self {
        let default = String::from("resources/OCEANMAN_UNSPECIFIED.dds");
        let cubemap_path = match &config.skybox {
            Some(x) => x,
            None => &default,
        };
        let cubemap = Cubemap::from_dds(device, queue, cubemap_path);
        let cubemap_sampler = Sampler::cubemap_sampler(device);

        let cubemap_bind_group_layout = Skybox::cubemap_bind_group_layout(device);
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/skybox.wgsl", true));
        let pipeline = Skybox::pipeline(device, &shader);

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

        Skybox {
            cubemap,
            cubemap_sampler,
            pipeline,
            cubemap_bind_group,
        }
    }

    pub fn update_cubemap(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_skybox: &String,
    ) {
        let cubemap = Cubemap::from_dds(device, queue, new_skybox);

        let cubemap_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("skybox bind group"),
            layout: &Skybox::cubemap_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cubemap.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.cubemap_sampler.sampler),
                },
            ],
        });

        self.cubemap.texture.destroy();

        self.cubemap = cubemap;
        self.cubemap_bind_group = cubemap_bind_group;
    }

    pub fn cubemap_bind_group_layout(device: &Device) -> BindGroupLayout {
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
        })
    }

    pub fn pipeline(device: &Device, shader_module: &ShaderModule) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Skybox pipeline layout"),
                bind_group_layouts: &[
                    &SceneUniform::bind_group_layout(device),
                    &Skybox::cubemap_bind_group_layout(device),
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
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
        })
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &scene.scene.uniform_bind_group, &[]);
            pass.set_bind_group(1, &self.cubemap_bind_group, &[]);
            pass.draw(0..36, 0..1);
        }
    }
}

impl ReloadableShaders for Skybox {
    fn available_shaders() -> &'static [&'static str] {
        &["../shaders/skybox.wgsl"]
    }

    fn reload(
        &mut self,
        device: &Device,
        _config: &wgpu::SurfaceConfiguration,
        shader_module: wgpu::ShaderModule,
    ) {
        self.pipeline = Skybox::pipeline(device, &shader_module);
    }
}
