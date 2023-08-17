use glam::{vec3, Mat4, Vec3};
use wgpu::{TextureFormat, TextureUsages};

use crate::{bytemuck_impl, texture::Texture, uniform::Uniform};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShadowData {
    pub view: Mat4,
    pub perspective: Mat4,
}
bytemuck_impl!(ShadowData);

impl ShadowData {
    /// pos should be camera pos, theta and phi are where direcitonal light originates from,
    /// looking from center of unit sphere.
    pub fn new(pos: Vec3, dist: f32, theta: f32, phi: f32) -> Self {
        let dir = -vec3(
            f32::sin(phi) * f32::cos(theta),
            f32::sin(phi) * f32::sin(theta),
            f32::cos(phi),
        );

        let up = vec3(0.0, 1.0, 0.0);

        let eye = pos + (-dir * dist);

        let view = Mat4::look_to_lh(eye, dir, up);
        let perspective = Mat4::perspective_lh(70.0, 1600.0 / 900.0, 0.01, 100.0);
        Self { view, perspective }
    }
}

pub struct Shadowmap {
    pub texture: Texture,
}

impl Shadowmap {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let map = Texture::new(
            device,
            config.width,
            config.height,
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            Some("Shadowmap"),
            false,
        );

        Self { texture: map }
    }
}

pub type ShadowUniform = Uniform<ShadowData>;

pub struct Shadows {
    pub shadowmap: Shadowmap,
    pub uniform: ShadowUniform,
}

impl Shadows {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let shadowmap = Shadowmap::new(device, config);
        let uniform = ShadowUniform::new(
            device,
            None,
            ShadowData::new(vec3(0.0, 0.0, 0.0), 10.0, 0.0, 0.0),
        );

        Self { shadowmap, uniform }
    }

    pub fn update_uniform(&mut self, queue: &wgpu::Queue, data: ShadowData) {
        self.uniform.update(queue, data);
    }
}
