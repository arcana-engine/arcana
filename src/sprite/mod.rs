mod anim;
// mod character;
mod graph;
mod sprite_sheet;

pub use self::{anim::*, graph::*, sprite_sheet::*};

use bytemuck::{Pod, Zeroable};

use crate::rect::Rect;

/// Sprite configuration.
///
/// |-------------|
/// | world       |
/// |  |--------| |
/// |  |src     | |
/// |  |        | |
/// |  |--------| |
/// |-------------|
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
#[repr(C)]
pub struct Sprite {
    /// Target rect to render this sprite into.
    pub world: Rect,

    /// Specifies fraction of `world` rect that will be occupied be texture.
    pub src: Rect,

    /// Cropped rect of the sprite's texture portion.
    pub tex: Rect,

    /// Layer at which sprite should be rendered
    /// The higher level sprites are rendered over
    /// lower layer sprites.
    pub layer: u32,
}

// struct Animation<F> {
//     pub from: usize,
//     pub to: usize,
//     pub looping: bool,
//     pub duration_us: u64,
//     pub features: F,
// }

// pub struct SpriteMatcher<F> {
//     frames: Arc<[SpriteFrame]>,
//     animations: Vec<Animation<F>>,
//     frame_distances: Arc<[f32]>,

//     current_animation: usize,
//     current_frame: usize,
//     frame_instant_us: u64,
//     scale: f32,
// }

// #[derive(Debug, thiserror::Error)]
// pub enum SpriteMatcherCreateError {
//     #[error("No animations in animation set")]
//     EmptyAnimSet,

//     #[error("Animation frame reference is out of bounds")]
//     AnimationOutOfBounds {
//         from: usize,
//         to: usize,
//         count: usize,
//     },

//     #[error("Failed to deserialize animation features")]
//     FeaturesDeserializationError { source: serde_json::Error },
// }

// /// Query matching metric for features.
// pub trait FrameQuery<F> {
//     /// Returns metric of how much features match this query.
//     fn matches(&self, features: &F) -> f32;

//     /// Returns scale required for features to match query as best as possible.
//     fn scale(&self, features: &F) -> f32;
// }

// impl<F> SpriteMatcher<F> {
//     pub fn new(sprite_sheet: SpriteSheet) -> Result<Self, SpriteMatcherCreateError>
//     where
//         F: serde::de::DeserializeOwned,
//     {
//         let animations = sprite_sheet
//             .animations
//             .iter()
//             .filter_map(|animation| {
//                 if animation.from.max(animation.to) >= sprite_sheet.frames.len() {
//                     return Some(Err(SpriteMatcherCreateError::AnimationOutOfBounds {
//                         from: animation.from,
//                         to: animation.to,
//                         count: sprite_sheet.frames.len(),
//                     }));
//                 }

//                 let looping = match animation.features.get("looping") {
//                     Some(serde_json::Value::Bool(looping)) => *looping,
//                     _ => false,
//                 };

//                 let features = match F::deserialize(&animation.features) {
//                     Ok(features) => features,
//                     Err(err) => {
//                         return Some(Err(
//                             SpriteMatcherCreateError::FeaturesDeserializationError { source: err },
//                         ))
//                     }
//                 };

//                 let duration_us = sprite_sheet.frames[animation.from..=animation.to]
//                     .iter()
//                     .map(|f| f.duration_us)
//                     .sum::<u64>()
//                     .min(1);

//                 Some(Ok(Animation {
//                     from: animation.from,
//                     to: animation.to,
//                     duration_us,
//                     looping,
//                     features,
//                 }))
//             })
//             .collect::<Result<Vec<_>, _>>()?;

//         if animations.is_empty() {
//             return Err(SpriteMatcherCreateError::EmptyAnimSet);
//         }

//         Ok(SpriteMatcher {
//             frames: sprite_sheet.frames,
//             animations,
//             frame_distances: sprite_sheet.frame_distances,
//             current_animation: 0,
//             current_frame: 0,
//             frame_instant_us: 0,
//             scale: 1.0,
//         })
//     }

//     /// Advances current animation.
//     pub fn advance(&mut self, delta: Duration) {
//         let delta = ((delta.as_secs_f32() * self.scale) * 1_000_000.0) as u64;
//         self.frame_instant_us += delta;

//         loop {
//             let frame = &self.frames[self.current_frame];
//             if frame.duration_us < self.frame_instant_us {
//                 return;
//             }

//             let animation = &self.animations[self.current_animation];

//             if animation.to == self.current_frame {
//                 if !animation.looping {
//                     // Freeze
//                     self.frame_instant_us = frame.duration_us;
//                     return;
//                 }

//                 self.frame_instant_us %= animation.duration_us;

//                 self.current_frame = animation.from;
//             } else {
//                 self.current_frame += 1;
//             }
//             self.frame_instant_us -= frame.duration_us;
//         }
//     }

//     pub fn query<Q>(&mut self, query: &Q) -> &SpriteFrame
//     where
//         Q: FrameQuery<F>,
//     {
//         let current_frame = &self.frames[self.current_frame];

//         let (animation_index, _, frame_index, _) = self
//             .animations
//             .iter()
//             .enumerate()
//             .map(|(index, animation)| {
//                 let offset = self.current_frame * self.frames.len();
//                 let mut min_distance_frame = animation.to;
//                 let mut min_distance = self
//                     .frame_distances
//                     .get(offset + animation.to)
//                     .copied()
//                     .unwrap_or(f32::INFINITY);

//                 for i in animation.from..animation.to {
//                     let frame_distance = self
//                         .frame_distances
//                         .get(offset + i)
//                         .copied()
//                         .unwrap_or(f32::INFINITY);
//                     if min_distance > frame_distance {
//                         min_distance = frame_distance;
//                         min_distance_frame = offset + i;
//                     }
//                 }

//                 (index, animation, min_distance_frame, min_distance)
//             })
//             .min_by_key(|&(_, animation, _, min_distance)| {
//                 OrderedFloat(query.matches(&animation.features) / min_distance)
//             })
//             .unwrap();

//         if animation_index == self.current_animation {
//             return current_frame;
//         }

//         self.current_animation = animation_index;
//         self.current_frame = frame_index;
//         &self.frames[frame_index]
//     }
// }
