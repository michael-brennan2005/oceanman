use tobj;
use wgpu::util::DeviceExt;

pub fn vertex_buffer_from_file(device: &wgpu::Device, path: String) -> wgpu::Buffer {
    let (models, _) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).unwrap();

    let mesh = &models[0].mesh;

    let mut vertex_vec: Vec<f32> = Vec::new();

    for i in 0..mesh.indices.len() {
        vertex_vec.push(mesh.positions[i * 3]);
        vertex_vec.push(mesh.positions[i * 3 + 1]);
        vertex_vec.push(mesh.positions[i * 3 + 2]);
        
        vertex_vec.push(mesh.normals[i * 3]);
        vertex_vec.push(mesh.normals[i * 3 + 1]);
        vertex_vec.push(mesh.normals[i * 3 + 2]);

        vertex_vec.push(mesh.texcoords[i * 2]);
        vertex_vec.push(mesh.texcoords[i * 2 + 1]);
    }

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex buffer"),
        contents: bytemuck::cast_slice(vertex_vec.as_slice()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX
    });

    buffer
}