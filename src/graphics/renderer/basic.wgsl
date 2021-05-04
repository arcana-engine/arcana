struct VertexSkinnedInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
    [[location(3)]] joints: vec4<u32>;
    [[location(4)]] weights: vec4<f32>;
};

struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] normal: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct Joint {
    value: mat4x4<f32>;
};

[[block]]
struct Uniforms {
    camera_view: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
    joints: [[stride(64)]] array<Joint, 128>;
    albedo_factor: vec3<f32>;
};

[[group(0), binding(0)]]
var sampler: sampler;

[[group(0), binding(1)]]
var albedo: texture_2d<f32>;

[[group(0), binding(2)]]
var uniform: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = uniform.camera_proj * uniform.camera_view * uniform.transform * vec4<f32>(in.pos, 1.0);
    out.uv = in.uv;
    out.normal = (vec4<f32>(in.normal, 0.0)).xyz;

    return out;
}

[[stage(vertex)]]
fn vs_skinned(
    in: VertexSkinnedInput,
) -> VertexOutput {
    var out: VertexOutput;

    let skin_x = uniform.joints[in.joints.x].value;
    let skin_y = uniform.joints[in.joints.y].value;
    let skin_z = uniform.joints[in.joints.z].value;
    let skin_w = uniform.joints[in.joints.w].value;

    let pos_x = skin_x * vec4<f32>(in.pos, 1.0);
    let pos_y = skin_y * vec4<f32>(in.pos, 1.0);
    let pos_z = skin_z * vec4<f32>(in.pos, 1.0);
    let pos_w = skin_w * vec4<f32>(in.pos, 1.0);

    let pos = pos_x * in.weights.x +
        pos_y * in.weights.y +
        pos_z * in.weights.z +
        pos_w * in.weights.w;

    out.pos = uniform.camera_proj * uniform.camera_view * uniform.transform * pos;
    out.uv = in.uv;
    out.normal = (uniform.transform * vec4<f32>(in.normal, 0.0)).xyz;

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let color = textureSample(albedo, sampler, in.uv).rgb;
    return vec4<f32>(color, 1.0);
    // return vec4<f32>(in.uv, 0.0, 1.0);
    //return vec4<f32>(in.normal * 0.5 + vec3<f32>(0.5, 0.5, 0.5), 1.0);
}
