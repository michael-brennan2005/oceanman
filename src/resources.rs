use wgpu::VertexBufferLayout;

fn vertex_buffer_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout { 
        array_stride: 8 * 4, 
        step_mode: wgpu::VertexStepMode::Vertex, 
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 3 * 4,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 6 * 4,
                shader_location: 2,
            },
        ] 
    }
}

pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer
}