use std::{num::NonZeroU32, path::Path, rc::Rc};

use glam::{vec3, Mat4};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BufferBindingType, BufferDescriptor, BufferUsages, Color, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipelineDescriptor, ShaderStages,
    TextureUsages, TextureView, VertexState,
};

use crate::texture::{Sampler, Texture};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Mat4Wrapped {
    x: Mat4,
}
unsafe impl bytemuck::Pod for Mat4Wrapped {}
unsafe impl bytemuck::Zeroable for Mat4Wrapped {}

pub struct Cubemap {
    //    textures: [wgpu::Texture; 6], // [+X, -X, +Y, -Y, +Z, -Z]
    //    texture_view: wgpu::TextureView,
    //    texture_sampler: wgpu::BindGroup,
}

impl Cubemap {
    pub fn from_equirectangular<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Self {
        // TODO: do this cubemap stuff in rgba16float, 32bit is overkill
        let img = image::open(path).unwrap().into_rgba32f();
        let width = img.width();
        let height = img.height();

        let equirectangular_texture = Texture::new_from_bytes(
            device,
            queue,
            bytemuck::cast_slice(img.into_vec().as_slice()),
            width,
            height,
            wgpu::TextureFormat::Rgba32Float,
            TextureUsages::all(),
            Some("Equirectangular HDR cubemap"),
        );
        let equirectangular_texture_sampler = Sampler::equirectangular_sampler(device);

        let eqr_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("eqr bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let eqr_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("eqr bind group"),
            layout: &eqr_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&equirectangular_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        &equirectangular_texture_sampler.sampler,
                    ),
                },
            ],
        });

        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("transform bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let transform_bind_groups = {
            let mut vec: Vec<BindGroup> = Vec::new();
            let mut views = vec![
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(1.0, 0.0, 0.0),
                    vec3(0.0, -1.0, 0.0),
                ),
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(-1.0, 0.0, 0.0),
                    vec3(0.0, -1.0, 0.0),
                ),
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 1.0, 0.0),
                    vec3(0.0, 0.0, 1.0),
                ),
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, -1.0, 0.0),
                    vec3(0.0, 0.0, -1.0),
                ),
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 0.0, 1.0),
                    vec3(0.0, -1.0, 0.0),
                ),
                Mat4::look_at_lh(
                    vec3(0.0, 0.0, 0.0),
                    vec3(0.0, 0.0, -1.0),
                    vec3(0.0, -1.0, 0.0),
                ),
            ];
            let perspective = Mat4::perspective_lh(90.0_f32.to_radians(), 1.0, 0.1, 10.0);

            for i in 0..6 {
                let buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[Mat4Wrapped {
                        x: perspective * views[i],
                    }]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &transform_bind_group_layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                vec.push(bind_group);
            }
            vec
        };

        let cube_textures = {
            let mut vec: Vec<Texture> = Vec::new();

            for i in 0..6 {
                let faces = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];
                vec.push(Texture::new(
                    device,
                    1024,
                    1024,
                    wgpu::TextureFormat::Rgba32Float,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    Some(format!("Cubemap texture - {} side", faces[i]).as_str()),
                ));
            }

            vec
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("E2C encoder "),
        });

        let shader = device.create_shader_module(include_wgsl!("shaders/cubemap/e2c.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&eqr_bind_group_layout, &transform_bind_group_layout],
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
                    format: wgpu::TextureFormat::Rgba32Float,
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

        for i in 0..6 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Cubemap pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &cube_textures[i].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            pass.set_bind_group(0, &eqr_bind_group, &[]);
            pass.set_bind_group(1, &transform_bind_groups[i], &[]);
            pass.draw(0..6, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Cubemap {}
    }
}
