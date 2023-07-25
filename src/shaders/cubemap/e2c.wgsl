@group(0) @binding(0) var eqr: texture_2d<f32>;

@group(1) @binding(0) var<uniform> mat: mat4x4<f32>;

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
	
	out.local_position = cube[in.index];
	out.clip_space_position = mat * vec4<f32>(cube[in.index], 1.0);	

	return out;
}

const invAtan = vec2<f32>(0.1591, 0.3183);

fn sample_spherical_map(v: vec3<f32>) -> vec2<f32> {
	var uv = vec2<f32>(atan2(v.z, v.x), asin(v.y));
	uv *= invAtan;
	uv += 0.5;
	return uv;
}

@fragment 
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let uv = sample_spherical_map(normalize(in.local_position));
	let color = textureLoad(eqr, vec2<i32>(uv * vec2<f32>(textureDimensions(eqr))), 0).rgb;

	return vec4(color, 1.0);
}
