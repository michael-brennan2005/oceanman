struct VertexInput {
    @location(0) position: vec3<f32>
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var ratio: f32 = 640.0 / 480.0;

    out.color = vec3<f32>(1.0,0.0,0.0);
    out.position = vec4<f32>(in.position.xyz, 1.0);

	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4<f32>(in.color, 1.0);
}