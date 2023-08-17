struct ShadowUniforms {
    view: mat4x4<f32>,
    perspective: mat4x4<f32>
}

@group(0) @binding(0) var<uniform> shadow: ShadowUniforms;

struct MeshUniforms {
    model: mat4x4<f32>
}

@group(1) @binding(0) var<uniform> mesh: MeshUniforms;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_space_position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_space_position = shadow.perspective * shadow.view * mesh.model * vec4<f32>(in.position, 1.0);
    return out;
}
