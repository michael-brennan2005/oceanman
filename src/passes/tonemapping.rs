use wgpu::{
    BindGroupLayout, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, ShaderModule, TextureView, VertexState,
};

use crate::texture::Texture;

use super::ReloadableShaders;

pub struct Tonemapping {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
}

impl Tonemapping {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        texture: &Texture,
    ) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/tonemapping.wgsl", true));

        let texture_bind_group_layout = Tonemapping::texture_bind_group_layout(device);
        let pipeline = Tonemapping::pipeline(device, config, &shader);

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            }],
        });

        Self {
            texture_bind_group,
            pipeline,
        }
    }

    pub fn texture_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        })
    }

    pub fn pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shader: &ShaderModule,
    ) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tonemap pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Tonemap pipeline layout"),
                bind_group_layouts: &[&Tonemapping::texture_bind_group_layout(device)],
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
                    format: config.format,
                    blend: None,
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
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },

            multiview: None,
        })
    }

    pub fn pass(&self, output: &TextureView, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tonemapping"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.texture_bind_group, &[]);

            pass.draw(0..6, 0..1);
        }
    }
}
impl ReloadableShaders for Tonemapping {
    fn available_shaders() -> &'static [&'static str] {
        &["../shaders/tonemapping.wgsl"]
    }

    fn reload(
        &mut self,
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        index: usize,
        shader_module: wgpu::ShaderModule,
    ) {
        self.pipeline = Tonemapping::pipeline(device, config, &shader_module);
    }
}
