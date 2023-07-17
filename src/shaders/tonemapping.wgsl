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

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {

	let color_hdr = textureLoad(
		input,
		vec2<i32>(floor(position.xy)),
		0 
	).rgb;

	let color_ldr = color_hdr / (color_hdr + vec3<f32>(1.0, 1.0, 1.0)); 

	// Output texture is Rgba8UnormSrgb, so this linear color will automatically be converted to sRGB
	return vec4<f32>(color_ldr, 1.0);
}
