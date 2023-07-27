struct SceneUniforms {
	perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_perspective_view: mat4x4<f32>,
	camera_pos: vec4<f32>
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@group(1) @binding(0) var skybox: texture_cube<f32>;
@group(1) @binding(1) var skybox_sampler: sampler;

struct VertexInput {
	@builtin(vertex_index) index: u32 // use this to index into cube buffer and thats the aPos
}

struct VertexOutput {
	@builtin(position) clip_space_position: vec4<f32>,
	@location(0) local_position: vec3<f32>, 
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
	var cube = array<vec3<f32>, 36>(vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>( 1.0,  1.0, -1.0),
	vec3<f32>( 1.0, -1.0, -1.0),
	vec3<f32>( 1.0,  1.0, -1.0),
	vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>(-1.0,  1.0, -1.0),
	vec3<f32>(-1.0, -1.0,  1.0),
	vec3<f32>( 1.0, -1.0,  1.0),
	vec3<f32>( 1.0,  1.0,  1.0),
	vec3<f32>( 1.0,  1.0,  1.0),
	vec3<f32>(-1.0,  1.0,  1.0),
	vec3<f32>(-1.0, -1.0,  1.0),
	vec3<f32>(-1.0,  1.0,  1.0),
	vec3<f32>(-1.0,  1.0, -1.0),
	vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>(-1.0, -1.0,  1.0),
	vec3<f32>(-1.0,  1.0,  1.0),
	vec3<f32>( 1.0,  1.0,  1.0),
	vec3<f32>( 1.0, -1.0, -1.0),
	vec3<f32>( 1.0,  1.0, -1.0),
	vec3<f32>( 1.0, -1.0, -1.0),
	vec3<f32>( 1.0,  1.0,  1.0),
	vec3<f32>( 1.0, -1.0,  1.0),
	vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>( 1.0, -1.0, -1.0),
	vec3<f32>( 1.0, -1.0,  1.0),
	vec3<f32>( 1.0, -1.0,  1.0),
	vec3<f32>(-1.0, -1.0,  1.0),
	vec3<f32>(-1.0, -1.0, -1.0),
	vec3<f32>(-1.0,  1.0, -1.0),
	vec3<f32>( 1.0,  1.0 , 1.0),
	vec3<f32>( 1.0,  1.0, -1.0),
	vec3<f32>( 1.0,  1.0,  1.0),
	vec3<f32>(-1.0,  1.0, -1.0),
	vec3<f32>(-1.0,  1.0,  1.0));

	var out: VertexOutput;

	let rotationView = mat4x4<f32>(
		scene.view[0][0],
		scene.view[0][1],
		scene.view[0][2],
		0.0,
		scene.view[1][0],
		scene.view[1][1],
		scene.view[1][2],
		0.0,
		scene.view[2][0],
		scene.view[2][1],
		scene.view[2][2],
		0.0,
		0.0,
		0.0,
		0.0,
		0.0
	);	
	out.local_position = cube[in.index];
	out.clip_space_position = (scene.perspective * rotationView * vec4<f32>(cube[in.index], 1.0)).xyww;	

	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let color = textureSample(skybox, skybox_sampler, in.local_position).rgb;
	return vec4<f32>(color, 1.0);
}
