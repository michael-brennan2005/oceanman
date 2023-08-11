struct SceneUniforms {
	perspective: mat4x4<f32>,
    view: mat4x4<f32>,
	inverse_perspective_view: mat4x4<f32>,
	camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@group(1) @binding(0) var depth_t: texture_depth_2d;
@group(1) @binding(1) var normal_t: texture_2d<f32>;
@group(1) @binding(2) var sample_kernel_t: texture_2d<f32>;
@group(1) @binding(3) var random_noise_t: texture_2d<f32>;

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

fn framebuffer_to_uv(texcoord: vec2<f32>) -> vec2<f32> {
	return texcoord / vec2<f32>(textureDimensions(depth_t));
}

fn vs_position_from_depth(texcoord: vec2<f32>) -> vec3<f32> {
	let snapped_texcoords = vec2<u32>(texcoord);
	let uv = framebuffer_to_uv(texcoord);
	let z = textureLoad(depth_t, snapped_texcoords, 0);
	let x = uv.x * 2.0 - 1.0;
	let y = (1.0 - uv.y) * 2.0 - 1.0;

	let projected_pos = vec4<f32>(x, y, z, 1.0);
	let vs_pos = scene.view * scene.inverse_perspective_view * projected_pos;

	return vs_pos.xyz / vs_pos.www;
}

fn texcoord_wrap(texcoord: vec2<f32>, scale: vec2<f32>) -> vec2<u32> {
	return vec2<u32>(texcoord % scale);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
	let radius = 0.01;

	// Get view space position of the fragment as well as view space normal	
	let origin = vs_position_from_depth(position.xy);
	var normal = (scene.view * vec4<f32>(textureLoad(normal_t, vec2<u32>(position.xy), 0).xyz * 2.0 - 1.0, 1.0)).xyz;
	normal = normalize(normal);

	// Create TBN matrix for hemisphere using Gram-Schmidt
	let rotation = textureLoad(random_noise_t, texcoord_wrap(position.xy, vec2<f32>(4.0, 4.0)), 0).xyz;
	let tangent = normalize(rotation - normal * dot(rotation, normal));
	let bitangent = cross(normal, tangent);
	let tbn = mat3x3<f32>(tangent, bitangent, normal);

	var occlusion = 0.0;

	// Iterate through our sample table and use each to calculate an occlusion value
	for (var x = 0.0; x < 4.0; x += 1.0) {
		for (var y = 0.0; y < 4.0; y += 1.0) {
			// Load one of the samples, multiply by TBN to get its position in hemisphere/view-space,
			// Then add to origin to get our final sample position
			var sample = textureLoad(sample_kernel_t, vec2<u32>(u32(x), u32(y)), 0).xyz;
			sample = tbn * sample;
			sample = sample * radius + origin;

			// Convert the offset to framebuffer coordinates which we can use to sample
			var offset = vec4<f32>(sample, 1.0);
			offset = scene.perspective * offset;
			offset.x = (offset.x / offset.w) * 0.5 + 0.5;
			offset.y = (offset.y / offset.w) * -0.5 + 0.5;
			var sample_depth_coords = offset.xy * vec2<f32>(textureDimensions(depth_t));

			// Sample depth
			var sample_depth = textureLoad(depth_t, vec2<u32>(sample_depth_coords), 0);

			// Calculate occlusion
			var range_check = select(1.0, 0.0, abs(origin.z - sample_depth) < radius);
			occlusion += (select(0.0, 1.0, sample_depth <= sample.z)) * range_check; 		
		}
	}

	occlusion = 1.0 - (occlusion / 16.0);
	return vec4<f32>(occlusion, 0.0, 0.0, 1.0);
}

