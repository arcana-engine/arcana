
[[group(0), binding(0)]] var<storage, read> rgb: array<u8>;
[[group(0), binding(1)]] var rgba: texture_storage_2d<rgba8unorm, write>;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_invocation_id : vec3<u32>) {
}
