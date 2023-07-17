struct SceneUniforms {
	perspective_view: mat4x4<f32>,
	inverse_perspective_view: mat4x4<f32>,
	camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var albedo: texture_2d<f32>;
@group(1) @binding(2) var normal: texture_2d<f32>;
@group(1) @binding(3) var material: texture_2d<f32>;

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

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {

	let albedoo = textureLoad(
		albedo,
		vec2<i32>(floor(position.xy)),
		0
	);
	
	let depthh = textureLoad(
		depth,
		vec2<i32>(floor(position.xy)),
		0 
	);

	let bufferSize = textureDimensions(depth);
	let coordUV = position.xy / vec2<f32>(bufferSize);
	let position = screen_to_world_coord(coordUV, depthh);
	
	return vec4<f32>(albedoo.rgb,1.0);
}