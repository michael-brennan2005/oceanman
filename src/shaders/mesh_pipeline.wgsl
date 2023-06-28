struct SceneUniforms {
    perspective_view: mat4x4<f32>,
    camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var<uniform> shadow: SceneUniforms;

struct LightingUniforms {
    direction: vec4<f32>,
    color: vec4<f32>
}

@group(1) @binding(0) var<uniform> lighting: LightingUniforms;

struct MeshUniforms {
    model: mat4x4<f32>,
    normal: mat4x4<f32>
}

@group(2) @binding(0) var<uniform> mesh: MeshUniforms;
@group(2) @binding(1) var texture: texture_2d<f32>;

struct VertexInput {
    @builtin(vertex_index) index: u32,
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
    out.clip_space_position = scene.perspective_view * mesh.model * vec4<f32>(in.position, 1.0);
    out.world_position = (mesh.model * vec4<f32>(in.position, 1.0)).xyz;
    out.normal = (mesh.normal * vec4<f32>(in.normal, 0.0)).xyz;    
    out.uv = in.uv;
	return out;
}

// TODO: move these to an actual bind group
const k_diff = 1.0;
const k_spec = 1.0;
const k_amb = 0.05;
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var texelCoords = vec2<i32>(in.uv * vec2<f32>(textureDimensions(texture)));
    var surface_color = textureLoad(texture, texelCoords, 0).rgb;

    var light_dir = -1.0 * lighting.direction.xyz;
    var view_dir = normalize(scene.camera_pos.xyz - in.world_position);
    var normal = normalize(in.normal);
    var reflected = 2.0 * dot(light_dir, normal) * normal - light_dir;

    var ambient = k_amb * surface_color;
    var diffuse = k_diff * clamp(dot(light_dir, normal), 0.0, 1.0) * surface_color * lighting.color.xyz;
    var specular = k_spec * pow(clamp(dot(reflected, view_dir), 0.0, 1.0), 16.0) * lighting.color.xyz;

    if (clamp(dot(light_dir, normal), 0.0, 1.0) <= 0.0) {
        specular = vec3<f32>(0.0, 0.0, 0.0);
    }

    var color = ambient + diffuse + specular;
    
    return vec4<f32>(color.xyz, 1.0);
}