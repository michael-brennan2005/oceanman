struct SceneUniforms {
	perspective: mat4x4<f32>,
    view: mat4x4<f32>,
	inverse_perspective_view: mat4x4<f32>,
	camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@group(1) @binding(0) var depth_gb: texture_depth_2d;
@group(1) @binding(1) var albedo_gb: texture_2d<f32>;
@group(1) @binding(2) var normal_gb: texture_2d<f32>;
@group(1) @binding(3) var material_gb: texture_2d<f32>;

struct LightingUniforms {
	count: u32,
	colors: array<vec4<f32>, 16>,
	positions: array<vec4<f32>, 16>,
}

@group(2) @binding(0) var<uniform> lighting: LightingUniforms;

@group(3) @binding(0) var brdf_lut: texture_2d<f32>;
@group(3) @binding(1) var diffuse_irradiance: texture_cube<f32>;
@group(3) @binding(2) var specular_prefilter: texture_cube<f32>;
@group(3) @binding(3) var ibl_s: sampler;

fn screen_to_world_coord(coord: vec2<f32>, depth_sample: f32) -> vec3<f32> {
	let pos_clip = vec4<f32>(coord.x * 2.0 - 1.0, (1.0 - coord.y) * 2.0 - 1.0, depth_sample, 1.0);
	let pos_world_w = scene.inverse_perspective_view * pos_clip;
	let pos_world = pos_world_w.xyz / pos_world_w.www;
	return pos_world;
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
	var vertex_positions = array<vec2<f32>, 6>(
		vec2<f32>(-1.0, -1.0),
		vec2<f32>(1.0, 1.0),
		vec2<f32>(-1.0, 1.0),
		vec2<f32>(-1.0, -1.0),
		vec2<f32>(1.0, -1.0),
		vec2<f32>(1.0, 1.0)
	);
	
	return vec4<f32>(vertex_positions[index], 0.0, 1.0);
}

const PI = 3.1415926535;

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
	let a = roughness * roughness;
	let a2 = a * a;
	let nDotH = max(dot(n, h), 0.0);
	let nDotH2 = nDotH * nDotH;

	let nom = a2;
	var denom = (nDotH2 * (a2 - 1.0) + 1.0);
	denom = PI * denom * denom;

	return nom / denom;
}

fn schlick_ggx(nDotV: f32, roughness: f32) -> f32 {
	let r = roughness + 1.0;
	let k = (r * r) / 8.0;
	
	let nom = nDotV;
	let denom = nDotV * (1.0 - k) + k;

	return nom / denom;
}

fn smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
	let nDotV = max(dot(n, v), 0.0);
	let nDotL = max(dot(n, l), 0.0);

	let ggx1 = schlick_ggx(nDotV, roughness);
	let ggx2 = schlick_ggx(nDotL, roughness);

	return ggx1 * ggx2;
}

fn schlick_fresnel(cosTheta: f32, f0: vec3<f32>) -> vec3<f32> {
	return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cosTheta, 5.0);
}

fn schlick_fresnel_roughness(cosTheta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
	return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

// thank you learnopengl - PBR!!!!
@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
	let depth = textureLoad(
		depth_gb,
		vec2<i32>(floor(position.xy)),
		0 
	);

	
	let albedo = textureLoad(
		albedo_gb,
		vec2<i32>(floor(position.xy)),
		0
	).rgb; 
	
	var normal = textureLoad(
		normal_gb,
		vec2<i32>(floor(position.xy)),
		0
	).rgb;
	normal = normalize(normal - 0.5);
	 
	let material = textureLoad(
		material_gb,
		vec2<i32>(floor(position.xy)),
		0
	);
	let metalness = material.r;
	let roughness = max(0.01, material.g);
	
	let bufferSize = textureDimensions(depth_gb);
	let coordUV = position.xy / vec2<f32>(bufferSize);
	let position = screen_to_world_coord(coordUV, depth);

	let v = normalize(scene.camera_pos.xyz - position);
	let n = normal;
	
	var f0 = vec3<f32>(0.04, 0.04, 0.04);
	f0 = mix(f0, albedo,  metalness);
	
	var l0 = vec3<f32>(0.0, 0.0, 0.0);
	for (var i: u32 = 0u; i < lighting.count; i++) {
		let l = normalize(lighting.positions[i].xyz - position);
		let h = normalize(v + l);

		let distance = length(lighting.positions[i].xyz - position);
		let attenuation = 1.0 / (distance * distance);
		let radiance = lighting.colors[i].rgb * attenuation;

		// BRDF
		let ndf = distribution_ggx(n, h, roughness);
		let g = smith(n, v, l, roughness);
		let f = schlick_fresnel(max(dot(h, v), 0.0), f0);

		var kD = (vec3<f32>(1.0) - f) * (1.0 - metalness);
		let nDotL = max(dot(n, l), 0.0);
		
		let numerator = ndf * g * f;
		let denominator = max(4.0 * max(dot(n, v), 0.0) * nDotL, 0.0001);
		let specular = numerator / vec3<f32>(denominator);
		
		let Fd = albedo / PI;
		
		l0 += (kD * Fd + specular) * radiance * nDotL;
	}
	
	let nDotV = max(dot(n, v), 0.0);
	let r = reflect(-v, n);
	
	let kS = schlick_fresnel_roughness(nDotV, f0, roughness);
	let kD = (1.0 - kS) * (1.0 - metalness);

	let irradiance = textureSample(diffuse_irradiance, ibl_s, n).rgb;
	let diffuse = irradiance * albedo;

	let roughness_level = f32(textureNumLevels(specular_prefilter)) * roughness * (2.0 - roughness);
	let prefiltered_color = textureSampleLevel(specular_prefilter, ibl_s, r, roughness_level).rgb;
	let brdf = textureSample(brdf_lut, ibl_s, vec2<f32>(nDotV, roughness)).rg;
	let specular = prefiltered_color * (f0 * brdf.x + brdf.y);
	
	let ambient = (kD * diffuse + specular);
	
	let color = ambient + l0;
	
	if (depth == 1.0) {
		return vec4<f32>(0.0, 0.0, 0.0, 1.0);
	} 
	
	return vec4<f32>(color,1.0);
}