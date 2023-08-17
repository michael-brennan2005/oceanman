use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, ShaderStages, TextureFormat, TextureUsages,
    TextureViewDimension,
};

pub struct Texture {
    pub sample_type: wgpu::TextureSampleType,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new_from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: Option<&str>,
        debug: bool,
    ) -> Self {
        let dimensions = (width, height);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: if debug {
                &[TextureFormat::Rgba8UnormSrgb]
            } else {
                &[]
            },
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let bytes_per_pixel = match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Rgba8Unorm => 4,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            wgpu::TextureFormat::Rg16Float => 4,
            _ => panic!("Unsupported format: {:?}", format),
        };

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_pixel * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        let sample_type = match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => {
                wgpu::TextureSampleType::Float { filterable: true }
            }
            wgpu::TextureFormat::Rgba8Unorm => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rgba16Float => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rgba32Float => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rg16Float => wgpu::TextureSampleType::Float { filterable: true },
            Texture::DEPTH_FORMAT => wgpu::TextureSampleType::Depth,
            _ => panic!("Unsupported format: {:?}", format),
        };
        Self {
            sample_type,
            texture,
            view,
        }
    }

    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: Option<&str>,
        debug: bool,
    ) -> Self {
        let dimensions = (width, height);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: if debug {
                &[TextureFormat::Rgba8UnormSrgb]
            } else {
                &[]
            },
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            format: Some(format),
            ..Default::default()
        });

        let sample_type = match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => {
                wgpu::TextureSampleType::Float { filterable: true }
            }
            wgpu::TextureFormat::Rgba8Unorm => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rgba16Float => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rgba32Float => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::Rg16Float => wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureFormat::R16Float => wgpu::TextureSampleType::Float { filterable: true },
            Texture::DEPTH_FORMAT => wgpu::TextureSampleType::Depth,
            _ => panic!("Unsupported format: {:?}", format),
        };

        Self {
            sample_type,
            texture,
            view,
        }
    }

    pub fn new_1x1_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        format: TextureFormat,
        usage: TextureUsages,
        label: Option<&str>,
    ) -> Self {
        Texture::new_from_bytes(device, queue, data, 1, 1, format, usage, label, false)
    }

    pub fn new_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        debug: bool,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: if debug {
                &[TextureFormat::Rgba8UnormSrgb]
            } else {
                &[]
            },
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            sample_type: wgpu::TextureSampleType::Depth,
            texture,
            view,
        }
    }

    pub fn bind_group_entry(&self, i: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: i,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }

    pub fn bind_group_layout_entry(&self, i: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: i,
            // TODO: fine-grain control this?
            visibility: ShaderStages::FRAGMENT | ShaderStages::VERTEX,
            ty: wgpu::BindingType::Texture {
                sample_type: self.sample_type,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }
}

pub struct Sampler {
    pub sampler: wgpu::Sampler,
}

impl Sampler {
    pub fn diffuse_texture_sampler(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Diffuse texture sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self { sampler }
    }

    pub fn normal_texture_sampler(device: &wgpu::Device) -> Self {
        // TODO: will diffuse and normal samplers always be same? check in after mipmaps
        Sampler::diffuse_texture_sampler(device)
    }

    #[allow(dead_code)]
    pub fn shadow_map_sampler(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow map sampler (PCF)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::Less),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self { sampler }
    }

    pub fn cubemap_sampler(device: &wgpu::Device) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Cubemap texture sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 10.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self { sampler }
    }
}
