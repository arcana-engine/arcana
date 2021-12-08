#version 460

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in uint albedo;
layout(location = 3) in vec4 albedo_factor;

layout(location = 0) out vec2 uv_out;
layout(location = 1) out uint albedo_out;
layout(location = 2) out vec4 albedo_factor_out;

void main() {
    gl_Position = vec4(pos, 0, 1);
    uv_out = uv;
    albedo_out = albedo;
    albedo_factor_out = albedo_factor;
}
