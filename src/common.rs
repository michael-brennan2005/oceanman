use wgpu::{VertexAttribute, VertexBufferLayout};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VertexAttributes {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 3],
}

unsafe impl bytemuck::Pod for VertexAttributes {}
unsafe impl bytemuck::Zeroable for VertexAttributes {}

impl VertexAttributes {
    const BUFFER_LAYOUT: [VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3=> Float32x3];

    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexAttributes>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VertexAttributes::BUFFER_LAYOUT,
        }
    }
}
