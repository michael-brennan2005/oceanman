use glam::{vec3, Mat4, Quat, Vec3, Vec4};
use gltf::{buffer::Data, image::Format};
use mikktspace::generate_tangents;
use serde::{Deserialize, Serialize};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages,
};

use crate::{
    common::VertexAttributes,
    resources::{
        LightingUniform, LightingUniformData, Material, MaterialUniformData, Mesh, MeshUniformData,
        SceneUniform, SceneUniformData,
    },
    tangent_generation::TangentGenerator,
    texture::Texture,
};

pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub scene: SceneUniform,
    pub lighting: LightingUniform,
}

// TODO: make message contain a String and get rid of this lifetime b.s
#[derive(Debug)]
pub enum SceneLoadError<'a> {
    Message(&'a str),
}

impl Scene {
    pub fn load_mesh<'a>(
        device: &wgpu::Device,
        node: &gltf::Node,
        original_transform: Mat4,
        buffers: &Vec<Data>,
    ) -> Result<Vec<Mesh>, SceneLoadError<'a>> {
        let (translation, rotation, scale) = node.transform().decomposed();

        let rotation_fixed = [
            rotation[0],
            rotation[1],
            rotation[2] * -1.0,
            rotation[3] * -1.0,
        ];
        let translation_fixed = [translation[0], translation[1], translation[2] * -1.0];
        let transform = original_transform
            * Mat4::from_scale_rotation_translation(
                scale.into(),
                Quat::from_array(rotation_fixed),
                translation_fixed.into(),
            );

        let mut meshes: Vec<Mesh> = Vec::new();

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| {
                    if buffer.index() < buffers.len() {
                        Some(buffers[buffer.index()].0.as_slice())
                    } else {
                        None
                    }
                });
                let material = primitive.material().index().unwrap_or(0);

                let indices = reader
                    .read_indices()
                    .ok_or(SceneLoadError::Message("Couldn't load indices"))?
                    .into_u32()
                    .collect::<Vec<_>>();
                let positions = reader
                    .read_positions()
                    .ok_or(SceneLoadError::Message("Couldn't load positions"))?
                    .map(|pos| [pos[0], pos[1], pos[2] * -1.0]);
                let normals = reader
                    .read_normals()
                    .ok_or(SceneLoadError::Message("Couldn't load normals."))?
                    .map(|pos| [pos[0], pos[1], pos[2] * -1.0]);
                let uvs = reader
                    .read_tex_coords(0)
                    .ok_or(SceneLoadError::Message("Couldn't load uvs."))?
                    .into_f32();

                let tangents = reader.read_tangents();

                let mut vertices = if tangents.is_some() {
                    positions
                        .zip(normals.zip(tangents.clone().unwrap().zip(uvs)))
                        .map(|(position, (normal, (tangent, uv)))| VertexAttributes {
                            position,
                            normal,
                            uv,
                            tangent: [tangent[0], tangent[1], tangent[2] * -1.0], // TODO: investigate handedness
                        })
                        .collect::<Vec<_>>()
                } else {
                    positions
                        .zip(normals.zip(uvs))
                        .map(|(position, (normal, uv))| VertexAttributes {
                            position,
                            normal,
                            uv,
                            tangent: [0.0, 0.0, 0.0], // TODO: investigate handedness
                        })
                        .collect::<Vec<_>>()
                };

                if tangents.is_none() {
                    let mut tangent_generator = TangentGenerator {
                        vertices: vertices.clone(),
                        indices: indices.clone(),
                    };

                    generate_tangents(&mut tangent_generator);

                    vertices = tangent_generator.vertices;
                }

                // TODO: proper labels
                let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(
                        format!("Vertex buffer for {}", mesh.name().unwrap_or("")).as_str(),
                    ),
                    contents: bytemuck::cast_slice(vertices.as_slice()),
                    usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
                });

                let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(format!("Index buffer for {}", mesh.name().unwrap_or("")).as_str()),
                    contents: bytemuck::cast_slice(indices.as_slice()),
                    usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
                });

                meshes.push(Mesh::new(
                    device,
                    vertex_buffer,
                    index_buffer,
                    vertices.len() as u32,
                    indices.len() as u32,
                    MeshUniformData::new(transform),
                    material,
                    Some(format!("{}", mesh.name().unwrap_or("Mesh")).as_str()),
                ));
            }
        }

        for child in node.children() {
            meshes.append(&mut Scene::load_mesh(device, &child, transform, buffers)?);
        }

        Ok(meshes)
    }

    pub fn from_gltf<'a>(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
        path: String,
    ) -> Result<Self, SceneLoadError<'a>> {
        // TODO: proper error handling everywhere
        // TODO: proper labeling everywhere
        let (document, buffers, images) = gltf::import(path).unwrap();

        let scene = document.default_scene().unwrap();

        let mut meshes: Vec<Mesh> = Vec::new();
        let mut materials: Vec<Material> = Vec::new();

        // TODO: get this to visit full node tree
        for node in scene.nodes() {
            meshes.append(&mut Scene::load_mesh(
                device,
                &node,
                Mat4::IDENTITY,
                &buffers,
            )?);
        }

        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();
            let material_data = MaterialUniformData {
                ambient: pbr.base_color_factor().into(),
                diffuse: {
                    let x = pbr.roughness_factor();
                    Vec4::splat(x)
                },
                specular: {
                    let x = pbr.metallic_factor();
                    Vec4::splat(x)
                },
            };

            let diffuse_texture = if let Some(texture_info) = pbr.base_color_texture() {
                let image = &images[texture_info.texture().source().index()];

                let mut data: Vec<u8> = Vec::new();
                println!("Diffuse format: {:?}", image.format);
                let slice = match image.format {
                    Format::R8G8B8A8 => image.pixels.as_slice(),
                    Format::R8G8B8 => {
                        for rgb in image.pixels.chunks(3) {
                            data.push(rgb[0]);
                            data.push(rgb[1]);
                            data.push(rgb[2]);
                            data.push(u8::MAX);
                        }
                        data.as_slice()
                    }
                    _ => todo!(),
                };

                Texture::create_from_bytes(
                    device,
                    queue,
                    slice,
                    image.width,
                    image.height,
                    Some(format!("Color texture for {}", material.name().unwrap_or("")).as_str()),
                    None,
                )
            } else {
                Texture::create_1x1_texture(
                    device,
                    queue,
                    &[255, 255, 255, 255],
                    Some(format!("Color texture for {}", material.name().unwrap_or("")).as_str()),
                    None,
                )
            };

            let normal_texture = if let Some(texture_info) = material.normal_texture() {
                let image = &images[texture_info.texture().source().index()];
                // TODO: we need the same thing of redoing the slice like in diffuse_texutre
                if image.format != Format::R8G8B8 {
                    println!("Found texture: {:?}", image.format);
                    return Err(SceneLoadError::Message("Wrong texture for normal."));
                }

                Texture::create_from_bytes(
                    device,
                    queue,
                    image.pixels.as_slice(),
                    image.width,
                    image.height,
                    Some(format!("Normal texture for {}", material.name().unwrap_or("")).as_str()),
                    Some(wgpu::TextureFormat::Rgba8Unorm),
                )
            } else {
                Texture::create_1x1_texture(
                    device,
                    queue,
                    &[128, 128, 255, 255],
                    Some(format!("Normal texture for {}", material.name().unwrap_or("")).as_str()),
                    Some(wgpu::TextureFormat::Rgba8Unorm),
                )
            };

            materials.push(Material::new(
                device,
                material_data,
                diffuse_texture,
                normal_texture,
            ));
        }

        let lighting_direction = Vec3::new(1.0, 0.0, 0.0);
        let lighting_color = Vec3::new(1.0, 1.0, 0.85);
        Ok(Self {
            meshes,
            materials,
            scene: SceneUniform::new(
                device,
                SceneUniformData::new(),
                SceneUniformData::shadow(vec3(100.0, 100.0, 100.0), lighting_direction),
            ),
            lighting: LightingUniform::new(
                device,
                config,
                LightingUniformData {
                    direction: (lighting_direction, 0.0).into(),
                    color: (lighting_color, 1.0).into(),
                },
            ),
        })
    }
}
