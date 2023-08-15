use wgpu::{TextureFormat, TextureUsages};

use crate::texture::Texture;

pub struct Shadowmap {
    pub texture: Texture,
}

impl Shadowmap {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let map = Texture::new(
            device,
            config.width,
            config.height,
            TextureFormat::Depth24Plus,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Shadowmap"),
        );

        Self { texture: map }
    }
}
