use wgpu::{BindGroupEntry, TextureUsages};

use crate::texture::Texture;

pub struct GBuffers {
    /// Depth buffer (used to calculate world position), Depth24Plus
    pub depth: Texture,
    /// Albedo of fragment, RGBA8
    pub albedo: Texture,
    /// Normal of fragment (world space), RGBA8
    pub normal: Texture,
    /// Material of fragment, RGBA8
    pub material: Texture,

    pub bind_group: wgpu::BindGroup,
}

impl GBuffers {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let depth = Texture::new_depth_texture(device, config);

        let albedo = Texture::new(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Gbuffers - albedo"),
        );

        let normal = Texture::new(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba8Unorm,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Gbuffers - normal (worldspace)"),
        );

        let material = Texture::new(
            device,
            config.width,
            config.height,
            wgpu::TextureFormat::Rgba8Unorm,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Gbuffers - material"),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compose - gbuffers bind group"),
            layout: &GBuffers::bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&albedo.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&material.view),
                },
            ],
        });

        Self {
            depth,
            albedo,
            normal,
            material,
            bind_group,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gbuffers bind group layout"),
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
}
