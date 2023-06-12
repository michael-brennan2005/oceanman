struct Uniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec3<f32>
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;


struct LightingUniforms {
    origins: array<vec3<f32>, 16>,
    colors: array<vec3<f32>, 16>,
    len: u32,
}

@group(1) @binding(0) var<uniform> lighting: LightingUniforms;

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
    
    out.normal = (uniforms.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_space_position = uniforms.perspective * uniforms.view * uniforms.model * vec4<f32>(in.position.xyz, 1.0);
    out.world_position = (uniforms.model * vec4<f32>(in.position,1.0)).xyz;
	return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var light_color = lighting.colors[0];
    return vec4<f32>( lighting.colors[0].xyz, 1.0);
}