#version 460

layout(location = 0) in vec4 pos_aabb;
layout(location = 1) in vec4 uv_aabb;
layout(location = 2) in uint layer;
layout(location = 3) in uint albedo;
layout(location = 4) in vec4 albedo_factor;
layout(location = 5) in mat3 tr;

layout(location = 0) out vec2 uv_out;
layout(location = 1) out uint albedo_out;
layout(location = 2) out vec4 albedo_factor_out;

layout(set = 0, binding = 2) uniform Uniforms {
    mat3 camera;
};

vec2 pt_from_aabb(vec4 aabb) {
    float xs[6] = { aabb.x, aabb.x, aabb.y, aabb.y, aabb.y, aabb.x };
    float ys[6] = { aabb.w, aabb.z, aabb.z, aabb.z, aabb.w, aabb.w };
    float x = xs[gl_VertexIndex];
    float y = ys[gl_VertexIndex];
    return vec2(x, y);
}

void main() {
    vec2 pos = pt_from_aabb(pos_aabb);
    vec2 uv = pt_from_aabb(uv_aabb);
    vec2 global = (camera * tr * vec3(pos, 1)).xy;
    gl_Position = vec4(global.xy, layer * 0.00001525902189669642, 1);

    albedo_out = albedo;
    albedo_factor_out = albedo_factor;
    uv_out = uv;
}
