use std::f32::consts::PI;

use glam::vec3;
use half::f16;
use rand::prelude::*;
use wgpu::{
    BindGroupLayout, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, ShaderModule, TextureFormat, TextureUsages, TextureView,
    VertexState,
};

use crate::{
    gbuffers::GBuffers,
    loader::Scene,
    resources::SceneUniform,
    texture::{Sampler, Texture},
};

use super::ReloadableShaders;

pub struct SSAO {
    pipeline: wgpu::RenderPipeline,
    textures_bind_group: wgpu::BindGroup,
    sample_kernel: Texture,
    random_noise: Texture,
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    v0 + t * (v1 - v0)
}

impl SSAO {
    pub fn sample_kernel_16x1() -> Vec<f16> {
        let mut vec: Vec<f16> = Vec::new();
        let mut rng = rand::thread_rng();

        for i in 0..16 {
            let mut sample = vec3(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(0.0..1.0),
            );
            sample = sample.normalize();

            let mut scale: f32 = i as f32 / 16.0;
            scale = lerp(0.1, 1.0, scale * scale);
            sample *= scale;

            vec.push(f16::from_f32(sample.x));
            vec.push(f16::from_f32(sample.y));
            vec.push(f16::from_f32(sample.z));
            vec.push(f16::from_f32(1.0)); // texture is rgba so 4th component
        }

        vec
    }

    pub fn random_noise_16x1() -> Vec<f16> {
        let mut vec: Vec<f16> = Vec::new();
        let mut rng = rand::thread_rng();

        for i in 0..16 {
            let mut noise = vec3(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), 0.0);
            noise = noise.normalize();

            vec.push(f16::from_f32(noise.x));
            vec.push(f16::from_f32(noise.y));
            vec.push(f16::from_f32(noise.z));
            vec.push(f16::from_f32(1.0)); // texture is rgba so 4th component
        }

        vec
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, gbuffers: &GBuffers) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/ssao.wgsl", true));
        let pipeline = SSAO::pipeline(device, &shader);

        let sample_kernel_data = SSAO::sample_kernel_16x1();
        let sample_kernel = Texture::new_from_bytes(
            device,
            queue,
            bytemuck::cast_slice(sample_kernel_data.as_slice()),
            4,
            4,
            TextureFormat::Rgba16Float,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            Some("Sample kernel (16x1)"),
            false,
        );

        let random_noise_data = SSAO::random_noise_16x1();
        let random_noise = Texture::new_from_bytes(
            device,
            queue,
            bytemuck::cast_slice(sample_kernel_data.as_slice()),
            4,
            4,
            TextureFormat::Rgba16Float,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            Some("Random noise (16x1)"),
            false,
        );

        let textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &SSAO::textures_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffers.depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffers.normal.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&sample_kernel.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&random_noise.view),
                },
            ],
        });

        Self {
            pipeline,
            textures_bind_group,
            sample_kernel,
            random_noise,
        }
    }

    pub fn textures_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn pipeline(device: &wgpu::Device, shader: &ShaderModule) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("SSAO pipeline layout"),
                bind_group_layouts: &[
                    &SceneUniform::bind_group_layout(device),
                    &SSAO::textures_bind_group_layout(device),
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

    pub fn pass(&self, scene: &Scene, output: &TextureView, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO"),
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
            pass.set_bind_group(0, &scene.scene.uniform_bind_group, &[]);
            pass.set_bind_group(1, &self.textures_bind_group, &[]);

            pass.draw(0..6, 0..1);
        }
    }
}

impl ReloadableShaders for SSAO {
    fn available_shaders() -> &'static [&'static str] {
        &["../shaders/ssao.wgsl"]
    }

    fn reload(
        &mut self,
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        index: usize,
        shader_module: wgpu::ShaderModule,
    ) {
        self.pipeline = SSAO::pipeline(device, &shader_module);
    }
}
