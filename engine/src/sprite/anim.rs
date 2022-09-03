use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use edict::{system::Res, world::QueryRef};

use crate::{clocks::ClockIndex, rect::Rect};

use super::{
    graph::{AnimGraph, AnimGraphState, AnimNode, AnimTransitionRule, Transition},
    Sprite, SpriteFrame, SpriteSheet, SpriteSize,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FaceDirection {
    Left,
    Right,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FrameSpan {
    from: usize,
    to: usize,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpriteGraphAnimation<R> {
    frames: Arc<[SpriteFrame]>,
    tex_size: SpriteSize,
    graph: Arc<AnimGraph<FrameSpan, R>>,
    state: AnimGraphState,
}

#[derive(Debug, thiserror::Error)]
pub enum SpriteAnimationError<'a> {
    #[error("Failed to find animation by name")]
    AnimationNotFound(Cow<'a, str>),
}

impl<'a> SpriteAnimationError<'a> {
    pub fn into_owned(self) -> SpriteAnimationError<'static> {
        match self {
            SpriteAnimationError::AnimationNotFound(name) => {
                SpriteAnimationError::AnimationNotFound(Cow::Owned(name.into_owned()))
            }
        }
    }
}

impl<R> SpriteGraphAnimation<R> {
    pub fn new<'a>(
        entry_animation: &'a str,
        sheet: &SpriteSheet,
        transitions: Vec<(R, Option<Vec<&str>>, &'a str)>,
    ) -> Result<Self, SpriteAnimationError<'a>> {
        let entry_animation = sheet
            .animations
            .iter()
            .position(|a| *a.name == *entry_animation)
            .ok_or(SpriteAnimationError::AnimationNotFound(
                entry_animation.into(),
            ))?;

        let graph = Arc::new(AnimGraph {
            animations: sheet
                .animations
                .iter()
                .map(|a| AnimNode {
                    animation: FrameSpan {
                        from: a.from,
                        to: a.to,
                    },
                    span: sheet.frames[a.from..=a.to].iter().map(|f| f.span).sum(),
                    transitions: transitions
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, (_, from, _))| match from {
                            None => Some(idx),
                            Some(from) => {
                                if from.contains(&&*a.name) {
                                    Some(idx)
                                } else {
                                    None
                                }
                            }
                        })
                        .collect(),
                })
                .collect(),
            transitions: transitions
                .into_iter()
                .map(|(rule, _, to)| {
                    Ok(Transition {
                        rule,
                        target: sheet
                            .animations
                            .iter()
                            .position(|a| *a.name == *to)
                            .ok_or(SpriteAnimationError::AnimationNotFound(to.into()))?,
                        transition: (),
                    })
                })
                .collect::<Result<_, _>>()?,
        });

        Ok(SpriteGraphAnimation {
            frames: sheet.frames.clone(),
            tex_size: sheet.tex_size,
            graph,
            state: AnimGraphState::new(entry_animation),
        })
    }
}

pub struct SpriteGraphAnimationSystem<S, R> {
    marker: PhantomData<fn() -> (S, R)>,
}

impl<S, R> Default for SpriteGraphAnimationSystem<S, R> {
    fn default() -> Self {
        SpriteGraphAnimationSystem::new()
    }
}

impl<S, R> SpriteGraphAnimationSystem<S, R> {
    pub const fn new() -> Self {
        SpriteGraphAnimationSystem {
            marker: PhantomData,
        }
    }
}

pub fn sprite_graph_animation_system<S, R>(
    query: QueryRef<(&S, &mut SpriteGraphAnimation<R>, &mut Sprite)>,
    clock: Res<ClockIndex>,
) where
    S: Send + Sync + 'static,
    R: AnimTransitionRule<S> + Send + Sync + 'static,
{
    let delta = clock.delta;
    query.for_each(|(state, anim, sprite)| {
        let result = anim.state.animate(state, &anim.graph, delta);
        let frames = &anim.frames[result.animation.from..=result.animation.to];

        let mut left = result.elapsed;

        let frame = frames
            .iter()
            .find(|frame| {
                if frame.span > left {
                    true
                } else {
                    left -= frame.span;
                    false
                }
            })
            .or_else(|| frames.last())
            .unwrap();

        sprite.src = Rect {
            left: (frame.src.x as f32) / frame.src_size.w as f32,
            right: (frame.src.x as f32 + frame.src.w as f32) / frame.src_size.w as f32,
            bottom: 1.0 - (frame.src.y as f32 + frame.src.h as f32) / frame.src_size.h as f32,
            top: 1.0 - (frame.src.y as f32) / frame.src_size.h as f32,
        };

        sprite.tex = Rect {
            left: (frame.tex.x as f32) / anim.tex_size.w as f32,
            right: (frame.tex.x as f32 + frame.tex.w as f32) / anim.tex_size.w as f32,
            bottom: (frame.tex.y as f32) / anim.tex_size.h as f32,
            top: (frame.tex.y as f32 + frame.tex.h as f32) / anim.tex_size.h as f32,
        };
    })
}
