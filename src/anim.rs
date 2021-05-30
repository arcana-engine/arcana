use {
    crate::{
        assets::{SpriteAnimation, SpriteFrame, SpriteSheet},
        graphics::Sprite,
        system::{System, SystemContext},
    },
    ordered_float::OrderedFloat,
    std::{sync::Arc, time::Duration},
};

pub struct SpriteAnimationSystem;

impl System for SpriteAnimationSystem {
    fn name(&self) -> &str {
        "SpriteAnimationSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        for (_, (state, sprite)) in cx.world.query_mut::<(&mut SpriteAnimState, &mut Sprite)>() {
            state.advance(cx.clock.delta);
            sprite.uv_dst = state.get_frame().dst;
            sprite.uv_src = state.get_frame().src;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct SpriteAnimState {
    current_animation: usize,
    current_frame: usize,
    current_frame_time_us: u64,
    anim: Anim,
    frames: Arc<[SpriteFrame]>,
    animations: Arc<[SpriteAnimation]>,
}

impl SpriteAnimState {
    fn new(sheet: &SpriteSheet) -> Self {
        SpriteAnimState {
            current_animation: 0,
            current_frame: 0,
            current_frame_time_us: 0,
            anim: Anim::Loop { animation: 0 },
            frames: sheet.frames.clone(),
            animations: sheet.animations.clone(),
        }
    }

    fn set_anim(&mut self, anim: Anim) {
        match anim {
            Anim::Loop { animation } => {
                self.anim = anim;
                self.current_animation = animation;
                self.current_frame = 0;
                self.current_frame_time_us = 0;
            }
            Anim::RunAndLoop { animation, .. } => {
                self.anim = anim;
                self.current_animation = animation;
                self.current_frame = 0;
                self.current_frame_time_us = 0;
            }
        }
    }

    fn get_frame(&self) -> &SpriteFrame {
        let anim = &self.animations[self.current_animation];
        &self.frames[anim.from..=anim.to][self.current_frame]
    }

    fn advance(&mut self, delta: Duration) {
        let mut delta = delta.as_micros() as u64;

        loop {
            let anim = &self.animations[self.current_animation];
            let frames = &self.frames[anim.from..=anim.to];

            if self.current_frame_time_us + delta < frames[self.current_frame].duration_us {
                self.current_frame_time_us += delta;
                return;
            }

            delta -= frames[self.current_frame].duration_us - self.current_frame_time_us;

            self.current_frame += 1;
            self.current_frame_time_us = 0;
            if frames.len() == self.current_frame {
                self.current_frame = 0;

                match self.anim {
                    Anim::Loop { .. } => {}
                    Anim::RunAndLoop { and_loop, .. } => {
                        self.anim = Anim::Loop {
                            animation: and_loop,
                        };
                        self.current_animation = and_loop;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Anim {
    /// Cycle through animations
    Loop {
        animation: usize,
    },
    RunAndLoop {
        animation: usize,
        and_loop: usize,
    },
}

struct Animation<F> {
    pub from: usize,
    pub to: usize,
    pub looping: bool,
    pub duration_us: u64,
    pub features: F,
}

pub struct SpriteMatcher<F> {
    frames: Arc<[SpriteFrame]>,
    animations: Vec<Animation<F>>,
    frame_distances: Arc<[f32]>,

    current_animation: usize,
    current_frame: usize,
    frame_instant_us: u64,
    scale: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum SpriteMatcherCreateError {
    #[error("No animations in animation set")]
    EmptyAnimSet,

    #[error("Animation frame reference is out of bounds")]
    AnimationOutOfBounds {
        from: usize,
        to: usize,
        count: usize,
    },

    #[error("Failed to deserialize animation features")]
    FeaturesDeserializationError { source: serde_json::Error },
}

/// Query matching metric for features.
pub trait FrameQuery<F> {
    /// Returns metric of how much features match this query.
    fn matches(&self, features: &F) -> f32;

    /// Returns scale required for features to match query as best as possible.
    fn scale(&self, features: &F) -> f32;
}

impl<F> SpriteMatcher<F> {
    pub fn new(sprite_sheet: SpriteSheet) -> Result<Self, SpriteMatcherCreateError>
    where
        F: serde::de::DeserializeOwned,
    {
        let animations = sprite_sheet
            .animations
            .iter()
            .filter_map(|animation| {
                if animation.from.max(animation.to) >= sprite_sheet.frames.len() {
                    return Some(Err(SpriteMatcherCreateError::AnimationOutOfBounds {
                        from: animation.from,
                        to: animation.to,
                        count: sprite_sheet.frames.len(),
                    }));
                }

                let looping = match animation.features.get("looping") {
                    Some(serde_json::Value::Bool(looping)) => *looping,
                    _ => false,
                };

                let features = match F::deserialize(&animation.features) {
                    Ok(features) => features,
                    Err(err) => {
                        return Some(Err(
                            SpriteMatcherCreateError::FeaturesDeserializationError { source: err },
                        ))
                    }
                };

                let duration_us = sprite_sheet.frames[animation.from..=animation.to]
                    .iter()
                    .map(|f| f.duration_us)
                    .sum::<u64>()
                    .min(1);

                Some(Ok(Animation {
                    from: animation.from,
                    to: animation.to,
                    duration_us,
                    looping,
                    features,
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if animations.is_empty() {
            return Err(SpriteMatcherCreateError::EmptyAnimSet);
        }

        Ok(SpriteMatcher {
            frames: sprite_sheet.frames,
            animations,
            frame_distances: sprite_sheet.frame_distances,
            current_animation: 0,
            current_frame: 0,
            frame_instant_us: 0,
            scale: 1.0,
        })
    }

    /// Advances current animation.
    pub fn advance(&mut self, delta: Duration) {
        let delta = ((delta.as_secs_f32() * self.scale) * 1_000_000.0) as u64;
        self.frame_instant_us += delta;

        loop {
            let frame = &self.frames[self.current_frame];
            if frame.duration_us < self.frame_instant_us {
                return;
            }

            let animation = &self.animations[self.current_animation];

            if animation.to == self.current_frame {
                if !animation.looping {
                    // Freeze
                    self.frame_instant_us = frame.duration_us;
                    return;
                }

                self.frame_instant_us %= animation.duration_us;

                self.current_frame = animation.from;
            } else {
                self.current_frame += 1;
            }
            self.frame_instant_us -= frame.duration_us;
        }
    }

    pub fn query<Q>(&mut self, query: &Q) -> &SpriteFrame
    where
        Q: FrameQuery<F>,
    {
        let current_frame = &self.frames[self.current_frame];

        let (animation_index, _, frame_index, _) = self
            .animations
            .iter()
            .enumerate()
            .map(|(index, animation)| {
                let offset = self.current_frame * self.frames.len();
                let mut min_distance_frame = animation.to;
                let mut min_distance = self
                    .frame_distances
                    .get(offset + animation.to)
                    .copied()
                    .unwrap_or(f32::INFINITY);

                for i in animation.from..animation.to {
                    let frame_distance = self
                        .frame_distances
                        .get(offset + i)
                        .copied()
                        .unwrap_or(f32::INFINITY);
                    if min_distance > frame_distance {
                        min_distance = frame_distance;
                        min_distance_frame = offset + i;
                    }
                }

                (index, animation, min_distance_frame, min_distance)
            })
            .min_by_key(|&(_, animation, _, min_distance)| {
                OrderedFloat(query.matches(&animation.features) / min_distance)
            })
            .unwrap();

        if animation_index == self.current_animation {
            return current_frame;
        }

        self.current_animation = animation_index;
        self.current_frame = frame_index;
        &self.frames[frame_index]
    }
}
