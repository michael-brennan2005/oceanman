use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, FragmentState,
    MultisampleState, PipelineLayoutDescriptor, PrimitiveState, ShaderModule, ShaderStages,
    TextureView, TextureViewDimension, VertexState,
};

use crate::{bytemuck_impl, texture::Texture, uniform::Uniform};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FxaaParams {
    edge_threshold: f32,
    edge_threshold_min: f32,
    subpix: f32,
    subpix_trim: f32,
    subpix_cap: f32,
    search_steps: f32,
    search_acceleration: f32,
    search_threshold: f32,
}
bytemuck_impl!(FxaaParams);

impl Default for FxaaParams {
    fn default() -> Self {
        FxaaParams {
            edge_threshold: 1.0 / 3.0,
            edge_threshold_min: 1.0 / 32.0,
            subpix: 0.0,
            subpix_trim: 1.0 / 2.0,
            subpix_cap: 1.0,
            search_steps: 5.0,
            search_acceleration: 1.0,
            search_threshold: 1.0 / 4.0,
        }
    }
}
pub type FxaaUniform = Uniform<FxaaParams>;

pub struct Fxaa {
    uniform: FxaaUniform,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl Fxaa {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        input_texture: &Texture,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/fxaa.wgsl", true));

        let uniform = FxaaUniform::new(device, None, FxaaParams::default());

        let pipeline = Fxaa::pipeline(device, config, &shader);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &Fxaa::bind_group_layout(device),
            entries: &[
                uniform.bind_group_entry(0),
                input_texture.bind_group_entry(1),
            ],
        });

        Self {
            uniform,
            bind_group,
            pipeline,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                FxaaUniform::bind_group_layout_entry(0),
                BindGroupLayoutEntry {
                    binding: 1,
                    // TODO: fine-grain control this?
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shader: &ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fxaa pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Fxaa pipeline layout"),
                bind_group_layouts: &[&Fxaa::bind_group_layout(device)],
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
                label: Some("FXAA"),
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
            pass.set_bind_group(0, &self.bind_group, &[]);

            pass.draw(0..6, 0..1);
        }
    }
}
