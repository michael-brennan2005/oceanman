use tobj;
use wgpu::util::DeviceExt;

pub fn vertex_buffer_from_file(device: &wgpu::Device, path: String) -> (wgpu::Buffer, u32) {
    let (models, _) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).unwrap();

    let mesh = &models[0].mesh;

    let mut vertex_vec: Vec<f32> = Vec::new();

    for i in &mesh.indices {
        vertex_vec.push(mesh.positions[*i as usize * 3]);
        vertex_vec.push(mesh.positions[*i as usize * 3 + 1]);
        vertex_vec.push(mesh.positions[*i as usize * 3 + 2]);

        if mesh.normals.is_empty() {
            vertex_vec.push(0.0);
            vertex_vec.push(0.0);
            vertex_vec.push(0.0);
        } else {
            vertex_vec.push(mesh.normals[*i as usize * 3]);
            vertex_vec.push(mesh.normals[*i as usize * 3 + 1]);
            vertex_vec.push(mesh.normals[*i as usize * 3 + 2]);
        }

        if mesh.texcoords.is_empty() {
            vertex_vec.push(0.0);
            vertex_vec.push(0.0);
        } else {
            vertex_vec.push(mesh.texcoords[*i as usize * 2]);
            vertex_vec.push(1.0 - mesh.texcoords[*i as usize * 2 + 1]);
        }
    }

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex buffer"),
        contents: bytemuck::cast_slice(vertex_vec.as_slice()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
    });

    (buffer, (vertex_vec.len() / 8) as u32)
}
