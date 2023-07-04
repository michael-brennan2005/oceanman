use std::{
    fmt, fs,
    path::{self, Path, PathBuf},
};

use glam::{vec3, vec4, EulerRot, Mat4, Quat, U64Vec4, Vec3};
use serde::{Deserialize, Serialize};
use tobj;
use wgpu::util::DeviceExt;

use crate::resources::{
    LightingUniform, LightingUniformData, Material, MaterialUniformData, Mesh, MeshUniformData,
    SceneUniform, SceneUniformData, Texture,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LightingConfig {
    direction: [f32; 3],
    color: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MeshConfig {
    obj: String,
    texture: Option<String>,
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    scale: Option<[f32; 3]>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SceneConfig {
    pub lighting: LightingConfig,
    pub mesh: MeshConfig,
}

pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub scene: SceneUniform,
    pub lighting: LightingUniform,
}

impl Scene {
    pub fn from_file(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
        path: String,
    ) -> Self {
        let bytes = fs::read(&path).unwrap();
        let deserialized: SceneConfig = serde_json::from_slice(&bytes).unwrap();

        let direction = Vec3::from_slice(&deserialized.lighting.direction).normalize();
        let color = Vec3::from_slice(&deserialized.lighting.color);

        let lighting = LightingUniform::new(
            device,
            config,
            LightingUniformData {
                direction: (direction, 0.0).into(),
                color: (color, 1.0).into(),
            },
        );

        let scene = SceneUniform::new(
            device,
            SceneUniformData::new(),
            SceneUniformData::shadow(vec3(100.0, 100.0, 100.0), direction),
        );

        // FIXME: tons of pathbufs and refs - any better way to do this?
        let mut root_path = PathBuf::from(&path);
        root_path.pop();

        let obj_path: PathBuf = [&root_path, &PathBuf::from(&deserialized.mesh.obj)]
            .iter()
            .collect();

        let texture = if let Some(texture_path_string) = deserialized.mesh.texture {
            let texture_path: PathBuf = [&root_path, &PathBuf::from(&texture_path_string)]
                .iter()
                .collect();

            let texture_bytes = fs::read(texture_path).unwrap();
            Texture::create_from_bytes(
                &device,
                &queue,
                texture_bytes.as_slice(),
                Some(&texture_path_string.as_str()),
            )
        } else {
            Texture::create_1x1_texture(&device, &queue, [255, 255, 255, 255], Some("1x1 texture"))
        };

        let scale = deserialized.mesh.scale.unwrap_or([1.0, 1.0, 1.0]);
        let position = deserialized.mesh.position.unwrap_or([0.0, 0.0, 0.0]);
        let rotation = {
            let xyz = deserialized.mesh.rotation.unwrap_or([0.0, 0.0, 0.0]);
            Quat::from_euler(EulerRot::XYZ, xyz[0], xyz[1], xyz[2])
        };

        let translation =
            Mat4::from_scale_rotation_translation(scale.into(), rotation, position.into());

        let (meshes, materials) = load_obj(device, queue, obj_path, translation, &root_path);
        Self {
            meshes,
            materials,
            scene,
            lighting,
        }
    }
}

pub fn load_obj<P: AsRef<Path> + fmt::Debug>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: P,
    translation: Mat4,
    root_path: &PathBuf,
) -> (Vec<Mesh>, Vec<Material>) {
    let (models, materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).unwrap();

    let mut meshes_vec: Vec<Mesh> = Vec::new();
    let mut materials_vec: Vec<Material> = Vec::new();

    for model in &models {
        let mesh = &model.mesh;
        let mut vertex_vec: Vec<f32> = Vec::new();

        for i in &mesh.indices {
            vertex_vec.push(mesh.positions[*i as usize * 3]);
            vertex_vec.push(mesh.positions[*i as usize * 3 + 1]);
            vertex_vec.push(mesh.positions[*i as usize * 3 + 2]);

            if mesh.normals.is_empty() {
                println!("RUST");
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(vertex_vec.as_slice()),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        });

        meshes_vec.push(Mesh::new(
            device,
            vertex_buffer,
            vertex_vec.len() as u32 / 8,
            MeshUniformData::new(translation),
            mesh.material_id.unwrap_or(0),
        ));
    }

    for material in &materials.unwrap() {
        let material_uniform_data = MaterialUniformData::new(
            material.ambient.unwrap_or([0.2, 0.2, 0.2]).into(),
            material.diffuse.unwrap_or([1.0, 1.0, 1.0]).into(),
            material.specular.unwrap_or([0.5, 0.5, 0.5]).into(),
        );

        let diffuse_texture = if let Some(diffuse_texture_path) = &material.diffuse_texture {
            let texture_path: PathBuf = [&root_path, &PathBuf::from(&diffuse_texture_path)]
                .iter()
                .collect();

            let texture_bytes = fs::read(texture_path).unwrap();
            Texture::create_from_bytes(
                &device,
                &queue,
                texture_bytes.as_slice(),
                Some(&diffuse_texture_path.as_str()),
            )
        } else {
            Texture::create_1x1_texture(device, queue, [255, 255, 255, 255], Some("1x1 texture"))
        };

        materials_vec.push(Material::new(
            device,
            material_uniform_data,
            diffuse_texture,
        ));
    }

    (meshes_vec, materials_vec)
}
