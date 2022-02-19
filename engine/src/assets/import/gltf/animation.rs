#[derive(serde::Serialize)]
enum SamplerOutput {
    Scalar(Vec<f32>),
    Vec2(Vec<[f32; 2]>),
    Vec3(Vec<[f32; 3]>),
    Vec4(Vec<[f32; 4]>),
}

#[derive(serde::Serialize)]
enum Property {
    Translation,
    Rotation,
    Scale,
    MorphTargetWeights,
}

#[derive(serde::Serialize)]
enum Interpolation {
    Linear,
    Step,
    CubicSpline,
}

#[derive(serde::Serialize)]
pub struct Channel {
    joint: usize,
    property: Property,
    input: Vec<f32>,
    output: SamplerOutput,
    interpolation: Interpolation,
}

#[derive(serde::Serialize)]
pub struct Animation {
    channels: Vec<Channel>,
}
