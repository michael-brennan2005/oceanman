use ddsfile::D3DFormat;
use half::f16;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, TextureFormat, TextureUsages, TextureView, VertexState,
};

use crate::{
    cubemap::Cubemap,
    gbuffers::GBuffers,
    loader::Scene,
    resources::{LightingUniform, SceneUniform},
    texture::{Sampler, Texture},
    RendererConfig,
};

pub struct IBL {
    brdf_lookup: Texture,
    diffuse_radiance: Cubemap,
    specular_radiance: Cubemap,
    cubemap_sampler: Sampler,
    bind_group: wgpu::BindGroup,
}

impl IBL {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer_config: &RendererConfig,
    ) -> Self {
        let brdf_lookup = {
            let file = std::fs::read("resources/OCEANMAN_BRDF.dds").unwrap();
            let img = ddsfile::Dds::read(file.as_slice()).unwrap();

            if img.get_d3d_format().unwrap() != D3DFormat::A32B32G32R32F {
                panic!("Format is {:?}", img.get_d3d_format());
            }

            let slice =
                &img.data.as_slice()[0..(4 * 4 * img.get_width() * img.get_height()) as usize];
            let bytes = slice
                .chunks(4)
                .map(|x| {
                    let elem_f32 = f32::from_le_bytes([x[0], x[1], x[2], x[3]]);
                    f16::from_f32(elem_f32)
                })
                .collect::<Vec<f16>>();

            Texture::new_from_bytes(
                device,
                queue,
                bytemuck::cast_slice(bytes.as_slice()),
                img.get_width(),
                img.get_height(),
                TextureFormat::Rgba16Float,
                TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                Some("BRDF lookup"),
            )
        };

        let diffuse_radiance = Cubemap::from_dds(device, queue, &renderer_config.irradiance);
        let specular_radiance = Cubemap::from_dds(device, queue, &renderer_config.prefilter);

        let cubemap_sampler = Sampler::cubemap_sampler(device);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("IBL bind group"),
            layout: &IBL::bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&brdf_lookup.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_radiance.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&specular_radiance.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&cubemap_sampler.sampler),
                },
            ],
        });

        Self {
            brdf_lookup,
            diffuse_radiance,
            specular_radiance,
            cubemap_sampler,
            bind_group,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("IBL bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}
pub struct Compose {
    ibl: IBL,
    pipeline: wgpu::RenderPipeline,
}

impl Compose {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer_config: &RendererConfig,
    ) -> Self {
        let ibl = IBL::new(device, queue, renderer_config);
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/compose.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Compose gbuffers pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Compose pipeline layout"),
                bind_group_layouts: &[
                    &SceneUniform::bind_group_layout(device),
                    &GBuffers::bind_group_layout(device),
                    &LightingUniform::bind_group_layout(device),
                    &IBL::bind_group_layout(device),
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
        });

        Self { ibl, pipeline }
    }

    pub fn pass(
        &self,
        scene: &Scene,
        gbuffers: &GBuffers,
        output: &TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Compose"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &scene.scene.uniform_bind_group, &[]);
            pass.set_bind_group(1, &gbuffers.bind_group, &[]);
            pass.set_bind_group(2, &scene.lighting.uniform_bind_group, &[]);
            pass.set_bind_group(3, &self.ibl.bind_group, &[]);

            pass.draw(0..6, 0..1);
        }
    }
}
