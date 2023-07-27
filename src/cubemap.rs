use std::{arch::x86_64::_XCR_XFEATURE_ENABLED_MASK, io::Read, path::Path};

use ddsfile::{Caps, Caps2, D3DFormat, PixelFormat};
use half::f16;
use wgpu::{
    Extent3d, ImageCopyTexture, TextureDescriptor, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

// TODO: this struct is identical to texture, do we wanna just have one texture type?
pub struct Cubemap {
    pub format: wgpu::TextureFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Cubemap {
    pub fn from_dds<P: AsRef<Path>>(device: &wgpu::Device, queue: &wgpu::Queue, path: P) -> Self {
        let file = std::fs::read(path).unwrap();
        let img = ddsfile::Dds::read(file.as_slice()).unwrap();

        if img.get_d3d_format().unwrap() != D3DFormat::A32B32G32R32F {
            panic!("Format is: {:?}", img.get_d3d_format());
        }

        if !img.header.caps2.contains(Caps2::CUBEMAP) {
            panic!("DDS needs cubemap");
        }

        let (width, height) = (img.get_width(), img.get_height());

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Cubemap texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 6,
            },
            mip_level_count: img.get_num_mipmap_levels(),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba16Float),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let vec_16f = {
            let mut vec: Vec<f16> = vec![];

            for i in 0..(img.data.len() / 4) {
                let elem_f32 = f32::from_le_bytes([
                    img.data[i * 4],
                    img.data[i * 4 + 1],
                    img.data[i * 4 + 2],
                    img.data[i * 4 + 3],
                ]);

                vec.push(f16::from_f32(elem_f32));
            }

            vec
        };

        let mut offset = 0;
        for face in 0..6 {
            for mip_map_lvl in 0..img.get_num_mipmap_levels() {
                let width_adjusted = width / (2_u32.pow(mip_map_lvl));
                let height_adjusted = height / (2_u32.pow(mip_map_lvl));
                let slice = &vec_16f.as_slice()
                    [offset..(offset + (4 * width_adjusted * height_adjusted) as usize)];

                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &texture,
                        mip_level: mip_map_lvl,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: face,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytemuck::cast_slice(slice),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(8 * width_adjusted),
                        rows_per_image: Some(height_adjusted),
                    },
                    wgpu::Extent3d {
                        width: width_adjusted,
                        height: height_adjusted,
                        depth_or_array_layers: 1,
                    },
                );
                offset += (4 * width_adjusted * height_adjusted) as usize;
            }
        }

        Cubemap {
            texture,
            view,
            format: wgpu::TextureFormat::Rgba16Float,
        }
    }
}
