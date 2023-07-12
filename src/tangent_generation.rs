use std::{cell::Cell, rc::Rc};

use mikktspace::{generate_tangents, Geometry};

use crate::common::VertexAttributes;

pub struct TangentGenerator {
    pub vertices: Vec<VertexAttributes>,
    pub indices: Vec<u32>,
}

impl Geometry for TangentGenerator {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        let index = self.indices[3 * face + vert] as usize;
        self.vertices[index].position
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        let index = self.indices[3 * face + vert] as usize;
        self.vertices[index].normal
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        let index = self.indices[3 * face + vert] as usize;
        self.vertices[index].uv
    }

    fn set_tangent_encoded(&mut self, _tangent: [f32; 4], _face: usize, _vert: usize) {
        let index = self.indices[3 * _face + _vert] as usize;
        self.vertices[index].tangent = [_tangent[0], _tangent[1], _tangent[2]];
    }
}
