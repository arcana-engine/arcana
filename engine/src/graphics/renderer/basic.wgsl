struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] norm: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

struct Uniforms {
    albedo_factor: vec4<f32>;
    camera_view: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
};

[[group(0), binding(0)]]
var albedo_sampler: sampler;

[[group(0), binding(1)]]
var albedo_texture: texture_2d<f32>;

[[group(0), binding(2)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = uniforms.camera_proj * uniforms.camera_view * uniforms.transform * vec4<f32>(in.pos, 1.0);
    out.uv = in.uv;

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let albedo = textureSample(albedo_texture, albedo_sampler, in.uv);
    return albedo * uniforms.albedo_factor;
}
