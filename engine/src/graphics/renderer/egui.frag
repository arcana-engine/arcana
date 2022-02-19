#version 460

layout(location = 0) in vec4 color_factor;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 color_out;

layout(set = 0, binding = 0) uniform sampler s;
layout(set = 0, binding = 1) uniform texture2D texture;

void main() {
    vec4 color = texture(sampler2D(texture, s), uv);
    color_out = color_factor * color;
}
