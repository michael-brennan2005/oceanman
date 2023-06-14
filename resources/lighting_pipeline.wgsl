struct SceneUniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    camera_pos: vec3<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct LightingUniforms {
    pos: vec4<f32>,
    color: vec4<f32>
}

@group(1) @binding(0) var<uniform> lighting: LightingUniforms;

struct MeshUniforms {
    model: mat4x4<f32>,
    normal: mat4x4<f32>
}

@group(2) @binding(0) var<uniform> mesh: MeshUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>
}

struct VertexOutput {
    @builtin(position) clip_space_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    out.normal = (mesh.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_space_position = scene.perspective * scene.view * mesh.model * vec4<f32>(in.position.xyz, 1.0);
    out.world_position = (mesh.model * vec4<f32>(in.position,1.0)).xyz;
	return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4<f32>(lighting.color.xyz, 1.0);
}