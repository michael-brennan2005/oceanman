use std::num::NonZeroU64;

pub struct BufferDesc<'a> {
    debug: Option<&'a str>,
    byte_size: u64,
    usage: Option<wgpu::BufferUsages>,
    data: &'a [u8],
}

pub struct Buffer {
    buffer: wgpu::Buffer,
}

pub enum TextureViewDimension {
    D2,
    Cube,
}

pub struct TextureDesc<'a> {
    debug: Option<&'a str>,
    dimensions: (u32, u32, u32),
    view_dimension: TextureViewDimension,
    format: wgpu::TextureFormat,
    usage: Option<wgpu::TextureUsages>,
    data: Option<&'a [u8]>,
}

pub struct Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

pub enum SamplerType {
    Filtering,
    Comparison,
}

pub struct SamplerDesc<'a> {
    debug: Option<&'a str>,
    sampler_type: SamplerType,
    compare_function: Option<wgpu::CompareFunction>,
    lod: f32,
}

pub struct Sampler {
    sampler: wgpu::Sampler,
}

pub enum TextureSampleType {
    Float,
    Depth,
}

pub struct BindGroupLayoutDesc<'a> {
    debug: Option<&'a str>,
    visibility: wgpu::ShaderStages,
    textures: &'a [(TextureSampleType, TextureViewDimension)],
    buffers: &'a [u64],
    samplers: &'a [SamplerType],
}

pub struct BindGroupLayout {
    bind_group_layout: wgpu::BindGroupLayout,
}

pub struct BindGroupDesc<'a> {
    debug: Option<&'a str>,
    layout: &'a BindGroupLayout,
    textures: &'a [&'a Texture],
    buffers: &'a [&'a Buffer],
    samplers: &'a [&'a Sampler],
}

pub struct BindGroup {
    bind_group: wgpu::BindGroup,
}
pub struct Handle(usize);

pub struct ResourceManager<'a> {
    // TODO: if everything can be moved to resource manager then these should be moves not refs
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> ResourceManager<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self { device, queue }
    }

    pub fn create_buffer(&self, desc: &BufferDesc) -> Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.debug,
            size: desc.byte_size,
            usage: desc.usage.unwrap_or(wgpu::BufferUsages::all()),
            mapped_at_creation: false,
        });

        self.queue.write_buffer(&buffer, 0, desc.data);

        Buffer { buffer }
    }

    pub fn create_texture(&self, desc: &TextureDesc) -> Texture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: desc.debug,
            size: wgpu::Extent3d {
                width: desc.dimensions.0,
                height: desc.dimensions.1,
                depth_or_array_layers: desc.dimensions.2,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: desc.usage.unwrap_or(wgpu::TextureUsages::all()),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: desc.debug,
            ..Default::default()
        });

        let bytes_per_pixel = match desc.format {
            wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Rgba8Unorm => 4,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            wgpu::TextureFormat::Rg16Float => 4,
            _ => panic!("Unsupported format: {:?}", desc.format),
        };

        if let Some(bytes) = desc.data {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_pixel * desc.dimensions.0),
                    rows_per_image: Some(desc.dimensions.1),
                },
                wgpu::Extent3d {
                    width: desc.dimensions.0,
                    height: desc.dimensions.1,
                    depth_or_array_layers: 1,
                },
            );
        }

        Texture { texture, view }
    }

    pub fn create_sampler(&self, desc: &SamplerDesc) -> Sampler {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.debug,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: desc.lod,
            compare: desc.compare_function,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Sampler { sampler }
    }

    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDesc) -> BindGroupLayout {
        let mut entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];

        let mut i = 0;
        for texture in desc.textures {
            let sample_type = match texture.0 {
                TextureSampleType::Float => wgpu::TextureSampleType::Float { filterable: true },
                TextureSampleType::Depth => wgpu::TextureSampleType::Depth,
            };

            let view_dimension = match texture.1 {
                TextureViewDimension::D2 => wgpu::TextureViewDimension::D2,
                TextureViewDimension::Cube => wgpu::TextureViewDimension::Cube,
            };

            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Texture {
                    sample_type,
                    view_dimension,
                    multisampled: false,
                },
                count: None,
            });
            i += 1;
        }

        for buffer in desc.buffers {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(*buffer),
                },
                count: None,
            });
            i += 1;
        }

        for sampler in desc.samplers {
            let sampler_type = match *sampler {
                SamplerType::Filtering => wgpu::SamplerBindingType::Filtering,
                SamplerType::Comparison => wgpu::SamplerBindingType::Comparison,
            };

            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: desc.visibility,
                ty: wgpu::BindingType::Sampler(sampler_type),
                count: None,
            });
        }

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: desc.debug,
                    entries: entries.as_slice(),
                });

        BindGroupLayout { bind_group_layout }
    }

    pub fn create_bind_group(&self, desc: &BindGroupDesc) -> BindGroup {
        let mut entries: Vec<wgpu::BindGroupEntry> = vec![];
        let mut i = 0;

        for texture in desc.textures {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            });
            i += 1;
        }

        for buffer in desc.buffers {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: buffer.buffer.as_entire_binding(),
            });
            i += 1;
        }

        for sampler in desc.samplers {
            entries.push(wgpu::BindGroupEntry {
                binding: i,
                resource: wgpu::BindingResource::Sampler(&sampler.sampler),
            });
            i += 1;
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: desc.debug,
            layout: &desc.layout.bind_group_layout,
            entries: entries.as_slice(),
        });

        BindGroup { bind_group }
    }
}
