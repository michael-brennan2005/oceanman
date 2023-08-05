mod compose;
mod skybox;
mod tonemapping;
mod write_gbuffers;

pub use compose::Compose;
pub use skybox::Skybox;
pub use tonemapping::Tonemapping;
use wgpu::Device;
pub use write_gbuffers::WriteGBuffers;

// TODO: gut instinct says this could be done better
pub trait ReloadableShaders {
    /// List of shaders a pass can have reloaded. Each tuple is shader; each string is
    /// one of the files pass uses (and can have reloaded).
    fn available_shaders() -> &'static [&'static str];
    /// Reload a shader. Passed in is shader_module (already validated!) that should be used.
    // TODO: hoooly shit there is way too much work involved in passing config to everything. only like one pass needs it for config.format, make it so config.format is a constant in project and remove this
    fn reload(
        &mut self,
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        index: usize,
        shader_module: wgpu::ShaderModule,
    );
}
