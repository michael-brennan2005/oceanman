use wgpu::FilterMode;

use crate::texture::Texture;

pub struct TextureDebug {
    pub view: wgpu::TextureView,
    pub id: egui::TextureId,
}

impl TextureDebug {
    pub fn new(
        device: &wgpu::Device,
        renderer: &mut egui_wgpu::Renderer,
        texture: &Texture,
    ) -> Self {
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            ..Default::default()
        });

        let id = renderer.register_native_texture(device, &view, FilterMode::Nearest);

        Self { view, id }
    }
}
