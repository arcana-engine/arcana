struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] joints: vec4<u32>;
    [[location(3)]] weights: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

struct Joint {
    value: mat4x4<f32>;
};

[[block]]
struct Uniforms {
    albedo_factor: vec4<f32>;
    camera_view: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
    joints: [[stride(64)]] array<Joint, 128>;
};

[[group(0), binding(0)]]
var uniform: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    const skin_x = uniform.joints[in.joints.x].value;
    const skin_y = uniform.joints[in.joints.y].value;
    const skin_z = uniform.joints[in.joints.z].value;
    const skin_w = uniform.joints[in.joints.w].value;

    const pos_x = skin_x * vec4<f32>(in.pos, 1.0);
    const pos_y = skin_y * vec4<f32>(in.pos, 1.0);
    const pos_z = skin_z * vec4<f32>(in.pos, 1.0);
    const pos_w = skin_w * vec4<f32>(in.pos, 1.0);

    const pos = pos_x * in.weights.x +
        pos_y * in.weights.y +
        pos_z * in.weights.z +
        pos_w * in.weights.w;

    out.pos = uniform.camera_proj * uniform.camera_view * uniform.transform * pos;
    out.color = vec4<f32>(in.normal * 0.5 + vec3<f32>(0.5, 0.5, 0.5), 1.0);

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
