struct Uniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec3<f32>
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct LightingUniforms {
    pos: vec4<f32>,
    color: vec4<f32>
}

@group(1) @binding(0) var<uniform> lighting: LightingUniforms;


struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>
}

struct VertexOutput {
    @builtin(position) clip_space_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    out.normal = (uniforms.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_space_position = uniforms.perspective * uniforms.view * uniforms.model * vec4<f32>(in.position.xyz, 1.0);
    out.world_position = (uniforms.model * vec4<f32>(in.position,1.0)).xyz;
    out.uv = in.uv;
	return out;
}

// Constants
// TODO: eventually change these into material and lighting constants? for diff lights, diff materials.
const diffuse_constant = 1.0;
const specular_constant = 1.0;
const ambient_constant = 0.05; 


@group(0) @binding(1) var texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var texelCoords = vec2<i32>(in.uv * vec2<f32>(textureDimensions(texture)));
    var surface_color = textureLoad(texture, texelCoords, 0).rgb;

    var light_dir = normalize(lighting.pos.xyz - in.world_position);
    var view_dir = normalize(uniforms.camera_pos - in.world_position);
    var normal = normalize(in.normal);
    var reflected = 2.0 * dot(light_dir, normal) * normal - light_dir;

    var ambient = ambient_constant * surface_color;
    var diffuse = diffuse_constant * clamp(dot(light_dir, normal), 0.0, 1.0) * surface_color * lighting.color.xyz;
    var specular = specular_constant * pow(clamp(dot(reflected, view_dir), 0.0, 1.0), 16.0) * lighting.color.xyz; 

    if (clamp(dot(light_dir, normal), 0.0, 1.0) <= 0.0) {
        specular = vec3<f32>(0.0, 0.0, 0.0);
    }
    var color = ambient + diffuse + specular;
    return vec4<f32>( color, 1.0);
}