use wgpu::TextureFormat;

// TODO: change all the create methods to be new, in line with rust idioms
pub struct Texture {
    pub format: wgpu::TextureFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// If format is left as None, then the format of the texture will be Rgba8UnormSrgb (color textures).
    pub fn create_from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
        format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let format = format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);
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
            // FIXME: putting all these usages can stop driver from doing optimizations, introduce more fined grain control.
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            ..Default::default()
        });

        let bytes_per_pixel = match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Rgba8Unorm => 4,
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

        Self {
            texture,
            view,
            format,
        }
    }

    pub fn create(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: Option<&str>,
        format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let format = format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);
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
            // FIXME: putting all these usages can stop driver from doing optimizations, introduce more fined grain control.
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            ..Default::default()
        });

        Self {
            texture,
            view,
            format,
        }
    }

    pub fn create_1x1_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        label: Option<&str>,
        format: Option<TextureFormat>,
    ) -> Self {
        Texture::create_from_bytes(device, queue, data, 1, 1, label, format)
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
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
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            format: Self::DEPTH_FORMAT,
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
}
