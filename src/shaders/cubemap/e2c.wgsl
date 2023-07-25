@group(0) @binding(0) var eqr: texture_2d<f32>;
@group(0) @binding(1) var eqr_s: sampler;

@group(1) @binding(0) var<uniform> mat: mat4x4<f32>;

struct VertexInput {
	index: global index // use this to index into cube buffer and thats the aPos
}

struct VertexOutput {
	localpos
	builtin pos
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
	localpos = apos;
	builting pos = mat * vec4<f32>(localPos, 1.0);	
}

#version 330 core
out vec4 FragColor;
in vec3 localPos;

uniform sampler2D equirectangularMap;

const vec2 invAtan = vec2(0.1591, 0.3183);
vec2 SampleSphericalMap(vec3 v)
{
    vec2 uv = vec2(atan(v.z, v.x), asin(v.y));
    uv *= invAtan;
    uv += 0.5;
    return uv;
}

void main()
{		
    vec2 uv = SampleSphericalMap(normalize(localPos)); // make sure to normalize localPos
    vec3 color = texture(equirectangularMap, uv).rgb;
    
    FragColor = vec4(color, 1.0);
}