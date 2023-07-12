struct SceneUniforms {
	perspective_view: mat4x4<f32>,
	camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@group(1) @binding(0) var world_position: texture_2d<f32>;
@group(1) @binding(1) var albedo: texture_2d<f32>;
@group(1) @binding(2) var normal: texture_2d<f32>;
@group(1) @binding(3) var material: texture_2d<f32>;

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
	var color = vec3<f32>(0.0,0.0,0.0);
	
	if position.x <= 533.0 {
		color = textureLoad(albedo, vec2<i32>(floor(position.xy)), 0).rgb;	
	} else if position.x <= 1066.0 {
		color = textureLoad(world_position, vec2<i32>(floor(position.xy)), 0).rgb;	
	} else {						
		color = textureLoad(normal, vec2<i32>(floor(position.xy)), 0).rgb;	
	} 	
	
	return vec4<f32>(color,1.0);
}