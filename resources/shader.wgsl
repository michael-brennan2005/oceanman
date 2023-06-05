struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

	if (in_vertex_index == 0u) {
		out.position = vec4<f32>(-0.5, -0.5, 0.0, 1.0);
        out.color = vec3<f32>(1.0, 0.0, 0.0);
	} else if (in_vertex_index == 1u) {
		out.position = vec4<f32>(0.5, -0.5, 0.0, 1.0);
        out.color = vec3<f32>(0.0, 1.0, 0.0);
	} else {
		out.position = vec4<f32>(0.0, 0.5, 0.0, 1.0);
        out.color = vec3<f32>(0.0, 0.0, 1.0);
	}
    
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4<f32>(in.color, 1.0);
}