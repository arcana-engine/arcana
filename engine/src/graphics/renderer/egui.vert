#version 460

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec2 out_uv;

layout(set = 0, binding = 1) uniform Uniforms {
    vec2 inv_dimensions;
};

vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

void main() {
    gl_Position = vec4(pos * inv_dimensions + vec2(-1, 1), 0, 1);
    out_color.rgb = srgb_to_linear(color.rgb);
    out_color.a = color.a;
    out_uv = uv;
}
