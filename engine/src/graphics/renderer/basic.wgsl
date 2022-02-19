struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
};

[[block]]
struct Uniforms {
    albedo_factor: vec4<f32>;
    camera_view: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
};

[[group(0), binding(2)]]
var uniform: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = uniform.camera_proj * uniform.camera_view * uniform.transform * vec4<f32>(in.pos, 1.0);

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
