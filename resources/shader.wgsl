struct Uniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
    
    out.normal = in.normal;
    out.clip_space_position = uniforms.perspective * uniforms.view * uniforms.model * vec4<f32>(in.position.xyz, 1.0);
    out.world_position = in.position;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var light_origin = vec3<f32>(-1.0, 10.0, 20.0);
    var light_color = vec3<f32>(1.0, 0.0, 0.0);
    var light_dir = normalize(light_origin - in.world_position);

    var surface_color = vec3<f32>(1.0, 1.0, 1.0);
    var color = dot(light_dir, normalize(in.normal)) * light_color * surface_color; 
    return vec4<f32>( color, 1.0);
}