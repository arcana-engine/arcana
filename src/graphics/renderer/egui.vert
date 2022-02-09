#version 460

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec2 out_uv;


layout(set = 0, binding = 2) uniform Uniforms {
    vec2 inv_dimensions;
};

void main() {
    gl_Position = vec4(pos * inv_dimensions + vec2(-1, 1), 0, 1);
    out_color = color;
    out_uv = uv;
}
