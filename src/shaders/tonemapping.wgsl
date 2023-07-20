@group(0) @binding(0) var input: texture_2d<f32>;

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

fn uncharted2_tonemap(x: vec3<f32>) -> vec3<f32> {
	let a = 0.15f;
	let b = 0.50f;
	let c = 0.10f;
	let d = 0.20f;
	let e = 0.02f;
	let f = 0.30f;
	return ((x * (a*x+c*b)+d*e)/(x*(a*x+b)+d*f)) - e/f;
}

fn uncharted2_filmic(x: vec3<f32>) -> vec3<f32> {
	let exposure_bias = 2.0f;
	let curr = uncharted2_tonemap(x * exposure_bias);

	let w = vec3<f32>(11.2, 11.2, 11.2);
	let white_scale = vec3<f32>(1.0, 1.0, 1.0) / uncharted2_tonemap(w);
	return curr * white_scale;
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {

	let color_hdr = textureLoad(
		input,
		vec2<i32>(floor(position.xy)),
		0 
	).rgb;

	let color_ldr = uncharted2_filmic(color_hdr); 

	// Output texture is Rgba8UnormSrgb, so this linear color will automatically be converted to sRGB
	return vec4<f32>(color_ldr, 1.0);
}
