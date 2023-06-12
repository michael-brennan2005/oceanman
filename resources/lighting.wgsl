struct Uniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec3<f32>
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
    
    out.normal = (uniforms.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_space_position = uniforms.perspective * uniforms.view * uniforms.model * vec4<f32>(in.position.xyz, 1.0);
    out.world_position = (uniforms.model * vec4<f32>(in.position,1.0)).xyz;
	return out;
}

// Constants
// TODO: eventually change these into material and lighting constants? for diff lights, diff materials.
const diffuse_constant = 0.7;
const specular_constant = 1.0;
const ambient_constant = 0.05; 

const light_origin = vec3<f32>(5.0, 5.0, -5.0);
const light_color = vec3<f32>(1.0, 1.0, 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var surface_color = vec3<f32>(1.0, 1.0, 1.0);

    var light_dir = normalize(light_origin - in.world_position);
    var view_dir = normalize(uniforms.camera_pos - in.world_position);
    var normal = normalize(in.normal);
    var reflected = 2.0 * dot(light_dir, normal) * normal - light_dir;

    var ambient = ambient_constant * surface_color;
    var diffuse = diffuse_constant * clamp(dot(light_dir, normal), 0.0, 1.0) * surface_color * light_color;
    var specular = specular_constant * pow(clamp(dot(reflected, view_dir), 0.0, 1.0), 16.0) * light_color; 

    if (clamp(dot(light_dir, normal), 0.0, 1.0) <= 0.0) {
        specular = vec3<f32>(0.0, 0.0, 0.0);
    }
    var color = ambient + diffuse + specular;
    return vec4<f32>( light_color, 1.0);
}