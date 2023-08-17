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
@group(0) @binding(2) var screen_s: sampler;

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

fn rgb_to_luma(rgb: vec3<f32>) -> f32 {
	return rgb.y * (0.587 / 0.299) + rgb.x;
}

fn fb_to_uv(pos: vec2<f32>) -> vec2<f32> {
	let rcp_frame = vec2<f32>(1.0 / 1600.0, 1.0 / 900.0);
	return pos.xy * rcp_frame.xy;
}

fn ts(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {
	return textureSample(t, s, uv);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
	var rcp_frame = vec2<f32>(1.0 / 1600.0, 1.0 / 900.0);
	// FXAA works in sRGB space but the -srgb textures were using do an automatic
	// sRGB -> linear convert in the shader. So we need to sample the srgb texture
	// and then do an linear -> sRGB conversion.
	var color = textureSample(screen, screen_s, fb_to_uv(position.xy)).rgb;
	color = pow(color, vec3<f32>(1.0 / 2.2));

	// Where to apply AA
	var luma = rgb_to_luma(color);

	var rgbN = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(0, -1)).rgb;
	var rgbW = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(-1, 0)).rgb;
	var rgbM = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(0,  0)).rgb;
	var rgbE = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(1,  0)).rgb;
	var rgbS = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(0,  1)).rgb;
	
	var lumaN = rgb_to_luma(rgbN);
	var lumaW = rgb_to_luma(rgbW);
	var lumaM = rgb_to_luma(rgbM);
	var lumaE = rgb_to_luma(rgbE);
	var lumaS = rgb_to_luma(rgbS);
	
	var range_min = min(lumaM, min(min(lumaN, lumaW), min(lumaS, lumaE)));
	var range_max = max(lumaM, max(max(lumaN, lumaW), max(lumaS, lumaE)));
	var range = range_max - range_min;
	
	// Estimate gradient and choose edge direction
	var rgbNW = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(-1, 1)).rgb;
	var rgbNE = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>( 1,-1)).rgb;
	var rgbSW = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>(-1, 1)).rgb;
	var rgbSE = textureSample(screen, screen_s, fb_to_uv(position.xy), vec2<i32>( 1, 1)).rgb;
	
	var lumaNW = rgb_to_luma(rgbNW);
	var lumaNE = rgb_to_luma(rgbNE);
	var lumaSW = rgb_to_luma(rgbSW);
	var lumaSE = rgb_to_luma(rgbSE);

	var lumaNS = lumaN + lumaS;
	var lumaWE = lumaW + lumaE;
	var luma_w_corners = lumaSW + lumaNW;
	var luma_s_corners = lumaSE + lumaSW;
	var luma_e_corners = lumaNE + lumaSE;
	var luma_n_corners = lumaNE + lumaNW;
	
	var edge_horizontal =	
		abs((0.25 * lumaNW) + (-0.5 * lumaW) + (0.25 * lumaSW)) +
		abs((0.50 * lumaN ) + (-1.0 * lumaM) + (0.50 * lumaS )) +
		abs((0.25 * lumaNE) + (-0.5 * lumaE) + (0.25 * lumaSE));
	var edge_vertical =	
		abs((0.25 * lumaNW) + (-0.5 * lumaN) + (0.25 * lumaNE)) +
		abs((0.50 * lumaW ) + (-1.0 * lumaM) + (0.50 * lumaE )) +
		abs((0.25 * lumaSW) + (-0.5 * lumaS) + (0.25 * lumaSE));

	var is_horizontal = (edge_horizontal >= edge_vertical);

	// Choosing edge orientation
	var luma1 = select(lumaW, lumaS, is_horizontal);
	var luma2 = select(lumaE, lumaN, is_horizontal);
	var gradient1 = luma1 - luma;
	var gradient2 = luma2 - luma;

	var is_1_steepest = abs(gradient1) >= abs(gradient2);
	var gradient_scaled = 0.25 * max(abs(gradient1), abs(gradient2));

	// Move half pixel and compute average luma
	var step_length = select(rcp_frame.x, rcp_frame.y, is_horizontal);
	var luma_local_average = 0.0;

	if (is_1_steepest) {
		step_length = -1.0 * step_length;
		luma_local_average = 0.5 * (luma1 + luma);
	} else {
		luma_local_average = 0.5 * (luma2 + luma);
	}

	var current_uv = fb_to_uv(position.xy);
	if (is_horizontal) {
		current_uv.y += step_length * 0.5;
	} else {
		current_uv.x += step_length * 0.5;
	}
	
	// First iteration exploration
	var offset = select(vec2<f32>(0.0, rcp_frame.y), vec2<f32>(rcp_frame.x, 0.0), is_horizontal);
	var uv1 = current_uv - offset;
	var uv2 = current_uv + offset;

	var luma_end_1 = rgb_to_luma(textureSample(screen, screen_s, uv1).rgb);
	var luma_end_2 = rgb_to_luma(textureSample(screen, screen_s, uv2).rgb);

	luma_end_1 -= luma_local_average;
	luma_end_2 -= luma_local_average;

	var reached1 = abs(luma_end_1) >= gradient_scaled;
	var reached2 = abs(luma_end_2) >= gradient_scaled;
	var reached_both = reached1 && reached2;

	if (!reached1) {
		uv1 -= offset;
	} 

	if (!reached2) {
		uv2 += offset;
	}

	// iterating
	if (!reached_both) {
		for (var i = 2u; i < 12u; i += 1u) {
			if (!reached1) {
				luma_end_1 = rgb_to_luma(ts(screen, screen_s, uv1).rgb);
				luma_end_1 = luma_end_1 - luma_local_average;
			}

			if (!reached2) {
				luma_end_2 = rgb_to_luma(ts(screen, screen_s, uv2).rgb);
				luma_end_2 = luma_end_2 - luma_local_average;
			}

			reached1 = abs(luma_end_1) >= gradient_scaled;
			reached2 = abs(luma_end_2) >= gradient_scaled;
			reached_both = reached1 && reached2;

			if (!reached1) {
				uv1 -= offset * params.search_acceleration;
			}

			if (!reached2) {
				uv2 += offset * params.search_acceleration;
			}

			if (reached_both) {
				break;
			}
		}
	}

	// Estimate offset
	var distance1 = select(fb_to_uv(position.xy).y - uv1.y, fb_to_uv(position.xy).x - uv1.x, is_horizontal);
	var distance2 = select(uv2.y - fb_to_uv(position.xy).y, uv2.x - fb_to_uv(position.xy).x, is_horizontal);

	var is_direction_1 = distance1 < distance2;
	var distance_final = min(distance1, distance2);

	var edge_thickness = (distance1 + distance2);
	var pixel_offset = distance_final / edge_thickness + 0.5;

	var is_luma_center_smaller = lumaM < luma_local_average;
	var correct_variation = (select(luma_end_2, luma_end_1, is_direction_1) < 0.0) != is_luma_center_smaller;
	var final_offset = select(0.0, pixel_offset, correct_variation);

	// Subpixel shifting
	var luma_average = (1.0 / 9.0) * (lumaNW + lumaN + lumaNE + lumaW + lumaM + lumaE + lumaSW + lumaS + lumaSE);
	var sub_pixel_offset_1 = clamp(abs(luma_average - lumaM) / range, 0.0, 1.0);
	var sub_pixel_offset_2 = (-2.0 * sub_pixel_offset_1 + 3.0) * sub_pixel_offset_1 * sub_pixel_offset_1;
	var sub_pixel_offset_final = sub_pixel_offset_2 * sub_pixel_offset_2 * 0.75;

	final_offset = max(final_offset, sub_pixel_offset_final);

	// Final UV coord
	var final_uv = fb_to_uv(position.xy);
	if (is_horizontal) {
		final_uv.y += final_offset * step_length;
	} else {
		final_uv.x += final_offset * step_length;
	}

	var final_color = ts(screen, screen_s, final_uv).rgb;
	
	if (range < max(params.edge_threshold_min, range_max * params.edge_threshold)) {
		// -srgb textures do an automatic linear -> sRGB at the end of the shader. We've been
		// working in sRGB space, so let's convert to linear.
		color = pow(color, vec3<f32>(2.2));
		return vec4<f32>(color, 1.0);
	}

	return vec4<f32>(final_color, 1.0);
}