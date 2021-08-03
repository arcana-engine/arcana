struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] norm: vec3<f32>;
    [[location(2)]] col: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] norm: vec3<f32>;
    [[location(1)]] col: vec4<f32>;
};

[[block]]
struct Uniforms {
    camera_view: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
};

[[group(0), binding(0)]]
var uniform: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = uniform.camera_proj * uniform.camera_view * uniform.transform * vec4<f32>(in.pos, 1.0);
    out.norm = (uniform.transform * vec4<f32>(in.norm, 0.0)).xyz;
    out.col = in.col;

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.col;
}
