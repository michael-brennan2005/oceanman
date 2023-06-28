use std::{
    fmt, fs,
    path::{self, Path, PathBuf},
};

use glam::{vec3, vec4, Mat4, U64Vec4, Vec3};
use serde::{Deserialize, Serialize};
use tobj;
use wgpu::util::DeviceExt;

use crate::resources::{
    LightingUniform, LightingUniformData, Mesh, MeshUniformData, SceneUniform, SceneUniformData,
    Texture,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LightingConfig {
    direction: [f32; 3],
    color: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MeshConfig {
    obj: String,
    texture: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SceneConfig {
    pub lighting: LightingConfig,
    pub mesh: MeshConfig,
}

pub struct Scene {
    pub mesh: Mesh,
    pub scene: SceneUniform,
    pub lighting: LightingUniform,
}

impl Scene {
    pub fn from_file(device: &wgpu::Device, queue: &wgpu::Queue, path: String) -> Self {
        let bytes = fs::read(&path).unwrap();
        let deserialized: SceneConfig = serde_json::from_slice(&bytes).unwrap();

        let direction = Vec3::from_slice(&deserialized.lighting.direction);
        let color = Vec3::from_slice(&deserialized.lighting.color);

        let lighting = LightingUniform::new(
            device,
            LightingUniformData {
                direction: (direction, 0.0).into(),
                color: (color, 1.0).into(),
            },
        );

        let scene = SceneUniform::new(
            device,
            SceneUniformData::new(),
            SceneUniformData::shadow(vec3(10.0, 10.0, 10.0), direction),
        );

        // FIXME: tons of pathbufs and refs - any better way to do this?
        let mut root_path = PathBuf::from(&path);
        root_path.pop();

        let obj_path: PathBuf = [&root_path, &PathBuf::from(&deserialized.mesh.obj)]
            .iter()
            .collect();
        let (vertex_buffer, vertex_count) = vertex_buffer_from_file(&device, obj_path);

        let texture_path: PathBuf = [&root_path, &PathBuf::from(deserialized.mesh.texture)]
            .iter()
            .collect();

        let texture_bytes = fs::read(texture_path).unwrap();
        let texture = Texture::create_from_bytes(
            &device,
            &queue,
            texture_bytes.as_slice(),
            Some("Mesh texture"),
        );
        let mesh = Mesh::new(
            &device,
            vertex_buffer,
            vertex_count,
            MeshUniformData::new(Mat4::from_scale(vec3(0.5, 0.5, 0.5))),
            texture,
        );

        Self {
            mesh,
            scene,
            lighting,
        }
    }
}

pub fn vertex_buffer_from_file<P: AsRef<Path> + fmt::Debug>(
    device: &wgpu::Device,
    path: P,
) -> (wgpu::Buffer, u32) {
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
