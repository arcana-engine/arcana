#version 460
#extension GL_EXT_nonuniform_qualifier : enable

layout(location = 0) in vec2 uv;
layout(location = 1) in flat uint albedo;
layout(location = 2) in vec3 albedo_factor;

layout(location = 0) out vec4 color_out;

layout(set = 0, binding = 0) uniform sampler s;
layout(set = 0, binding = 1) uniform texture2D textures[];

void main() {
    if (albedo != 0xFFFFFFFF) {
        color_out = texture(sampler2D(textures[albedo], s), uv) * vec4(albedo_factor, 1);
    } else {
        color_out = vec4(albedo_factor, 1);
    }
}
