struct FxaaParams {
    edge_threshold: f32,
    edge_threshold_min: f32,
    subpix: f32,
    subpix_trim: f32,
    subpix_cap: f32,
    search_steps: f32,
    search_acceleration: f32,
    search_threshold: f32,
}

@group(0) @binding(0) var<uniform> params: FxaaParams;
@group(0) @binding(1) var screen: texture_2d<f32>;

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
	return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}