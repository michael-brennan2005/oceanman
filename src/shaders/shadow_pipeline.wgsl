struct SceneUniforms {
    perspective_view: mat4x4<f32>,
    camera_pos: vec3<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var<uniform> shadow: SceneUniforms;

struct MeshUniforms {
    model: mat4x4<f32>,
    normal: mat4x4<f32>,
}

@group(2) @binding(0) var<uniform> mesh: MeshUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>
}

struct VertexOutput {
    @builtin(position) clip_space_position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_space_position = lighting.projection_matrix * mesh.model * vec4<f32>(in.position, 1.0) - vec4<f32>(0.0, 0.0, 1.0, 0.0);
	return out;
}

@fragment
fn fs_main(in: VertexOutput)  {}
