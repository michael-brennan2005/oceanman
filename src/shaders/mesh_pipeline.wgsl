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
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;

struct MaterialUniforms {
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    specular: vec4<f32>,
} 

@group(2) @binding(0) var<uniform> material: MaterialUniforms;
@group(2) @binding(1) var diffuse_texture: texture_2d<f32>;
@group(2) @binding(2) var diffuse_texture_sampler: sampler;
@group(2) @binding(3) var normal_texture: texture_2d<f32>;
@group(2) @binding(4) var normal_texture_sampler: sampler;

struct MeshUniforms {
    model: mat4x4<f32>,
}

@group(3) @binding(0) var<uniform> mesh: MeshUniforms;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_space_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
    @location(5) shadow_coord: vec3<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_space_position = scene.perspective_view * mesh.model * vec4<f32>(in.position, 1.0);
    out.world_position = (mesh.model * vec4<f32>(in.position, 1.0)).xyz;
    out.normal = (mesh.model * vec4<f32>(in.normal, 0.0)).xyz;    
    out.uv = in.uv;
    out.shadow_coord = (shadow.perspective_view * mesh.model * vec4<f32>(in.position, 1.0)).xyz;
	out.shadow_coord.x = out.shadow_coord.x * 0.5 + 0.5;
    out.shadow_coord.y = out.shadow_coord.y * -0.5 + 0.5;
    out.tangent = (mesh.model * vec4<f32>(in.tangent, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var surface_color = textureSample(diffuse_texture, diffuse_texture_sampler, in.uv).rgb;
    var normal = 
        textureSample(normal_texture, normal_texture_sampler, in.uv).rgb;
    normal = normal - 0.5;
    var rotation = mat3x3<f32>(
        normalize(in.tangent),
        normalize(cross(in.tangent, in.normal)),
        normalize(in.normal)
    );

    normal = in.normal; // TODO: we need to figure out some handedness and LH/RH stuff in gltf first before we can get back to normal mapping
    // percentage-closer filtering
    var increment = 1.0 / 1024.0;
    var visibility = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            var offset = vec2<f32>(vec2(x,y)) * increment;
            visibility += textureSampleCompare(shadow_map, shadow_sampler, in.shadow_coord.xy + offset, in.shadow_coord.z - .007);
        }
    }
    visibility /= 9.0;
    
    var light_dir = -1.0 * lighting.direction.xyz;
    var view_dir = normalize(scene.camera_pos.xyz - in.world_position);
    var reflected = 2.0 * dot(light_dir, normal) * normal - light_dir;

    var ambient = material.ambient.xyz * surface_color;
    var diffuse = material.diffuse.xyz * clamp(dot(light_dir, normal), 0.0, 1.0) * surface_color * lighting.color.xyz;
    var specular = material.specular.xyz * pow(clamp(dot(reflected, view_dir), 0.0, 1.0), 16.0) * lighting.color.xyz;

    if (clamp(dot(light_dir, normal), 0.0, 1.0) <= 0.0) {
        specular = vec3<f32>(0.0, 0.0, 0.0);
    }

    var color = ambient + visibility * (diffuse + specular);


    return vec4<f32>(color, 1.0);
}