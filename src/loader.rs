use glam::{vec3, Mat4, Quat, Vec3, Vec4};
use gltf::{buffer::Data, image::Format};
use mikktspace::generate_tangents;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, TextureUsages,
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

#[derive(Debug)]
pub enum SceneLoadError {
    Message(String),
}

impl Scene {
    pub fn load_mesh<'a>(
        device: &wgpu::Device,
        node: &gltf::Node,
        original_transform: Mat4,
        buffers: &Vec<Data>,
    ) -> Result<Vec<Mesh>, SceneLoadError> {
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
                    .ok_or(SceneLoadError::Message(String::from(
                        "Couldn't load indices",
                    )))?
                    .into_u32()
                    .collect::<Vec<_>>();
                let positions = reader
                    .read_positions()
                    .ok_or(SceneLoadError::Message(String::from(
                        "Couldn't load positions",
                    )))?
                    .map(|pos| [pos[0], pos[1], pos[2] * -1.0]);
                let normals = reader
                    .read_normals()
                    .ok_or(SceneLoadError::Message(String::from(
                        "Couldn't load normals.",
                    )))?
                    .map(|pos| [pos[0], pos[1], pos[2] * -1.0]);
                let uvs = reader
                    .read_tex_coords(0)
                    .ok_or(SceneLoadError::Message(String::from("Couldn't load uvs.")))?
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
        queue: &wgpu::Queue,
        path: &String,
    ) -> Result<Self, SceneLoadError> {
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

            let albedo_texture = if let Some(texture_info) = pbr.base_color_texture() {
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

                Texture::new_from_bytes(
                    device,
                    queue,
                    slice,
                    image.width,
                    image.height,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    Some(format!("Color texture for {}", material.name().unwrap_or("")).as_str()),
                )
            } else {
                Texture::new_1x1_texture(
                    device,
                    queue,
                    &[255, 255, 255, 255],
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    Some(format!("Color texture for {}", material.name().unwrap_or("")).as_str()),
                )
            };

            let normal_texture = if let Some(texture_info) = material.normal_texture() {
                let image = &images[texture_info.texture().source().index()];

                let mut data: Vec<u8> = Vec::new();
                println!("Normal format: {:?}", image.format);
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

                Texture::new_from_bytes(
                    device,
                    queue,
                    slice,
                    image.width,
                    image.height,
                    wgpu::TextureFormat::Rgba8Unorm,
                    TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    Some(format!("Normal texture for {}", material.name().unwrap_or("")).as_str()),
                )
            } else {
                Texture::new_1x1_texture(
                    device,
                    queue,
                    &[128, 128, 255, 255],
                    wgpu::TextureFormat::Rgba8Unorm,
                    TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    Some(format!("Normal texture for {}", material.name().unwrap_or("")).as_str()),
                )
            };

            let metal_roughness_texture =
                if let Some(texture_info) = pbr.metallic_roughness_texture() {
                    let image = &images[texture_info.texture().source().index()];

                    let mut data: Vec<u8> = Vec::new();
                    println!("Normal format: {:?}", image.format);
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

                    Texture::new_from_bytes(
                        device,
                        queue,
                        slice,
                        image.width,
                        image.height,
                        wgpu::TextureFormat::Rgba8Unorm,
                        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                        Some(
                            format!(
                                "Metallic-roughness texture for {}",
                                material.name().unwrap_or("")
                            )
                            .as_str(),
                        ),
                    )
                } else {
                    Texture::new_1x1_texture(
                        device,
                        queue,
                        &[255, 128, 128, 255],
                        wgpu::TextureFormat::Rgba8Unorm,
                        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                        Some(
                            format!(
                                "Metallic-roughness texture for {}",
                                material.name().unwrap_or("")
                            )
                            .as_str(),
                        ),
                    )
                };
            materials.push(Material::new(
                device,
                material_data,
                albedo_texture,
                normal_texture,
                metal_roughness_texture,
            ));
        }

        let lighting_direction = Vec3::new(1.0, 0.0, 0.0);
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
                LightingUniformData::new(vec![
                    (vec3(-4.0, 4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(4.0, 4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(-4.0, -4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(4.0, -4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                ]),
            ),
        })
    }

    pub fn new(device: &wgpu::Device) -> Self {
        let lighting_direction = Vec3::new(1.0, 0.0, 0.0);
        Self {
            meshes: vec![],
            materials: vec![],
            scene: SceneUniform::new(
                device,
                SceneUniformData::new(),
                SceneUniformData::shadow(vec3(100.0, 100.0, 100.0), lighting_direction),
            ),
            lighting: LightingUniform::new(
                device,
                LightingUniformData::new(vec![
                    (vec3(-4.0, 4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(4.0, 4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(-4.0, -4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                    (vec3(4.0, -4.0, -1.0), vec3(1.0, 1.0, 1.0)),
                ]),
            ),
        }
    }
}
