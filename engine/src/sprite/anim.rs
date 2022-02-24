use std::{marker::PhantomData, sync::Arc};

use crate::{
    graphics::sprite_sheet::{SpriteFrame, SpriteSheet, SpriteSize},
    rect::Rect,
    system::{System, SystemContext},
};

use super::{
    graph::{AnimGraph, AnimGraphState, AnimNode, AnimTransitionRule, Transition},
    Sprite,
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

impl<R> SpriteGraphAnimation<R> {
    pub fn new(
        entry_animation: usize,
        sheet: &SpriteSheet,
        transitions: Vec<(R, Vec<usize>, usize)>,
    ) -> Self {
        let graph = Arc::new(AnimGraph {
            animations: sheet
                .animations
                .iter()
                .enumerate()
                .map(|(i, a)| AnimNode {
                    animation: FrameSpan {
                        from: a.from,
                        to: a.to,
                    },
                    span: sheet.frames[a.from..=a.to].iter().map(|f| f.span).sum(),
                    transitions: transitions
                        .iter()
                        .enumerate()
                        .filter_map(
                            |(idx, (_, from, _))| {
                                if from.contains(&i) {
                                    Some(idx)
                                } else {
                                    None
                                }
                            },
                        )
                        .collect(),
                })
                .collect(),
            transitions: transitions
                .into_iter()
                .map(|(rule, _, to)| Transition {
                    rule,
                    target: to,
                    transition: (),
                })
                .collect(),
        });

        SpriteGraphAnimation {
            frames: sheet.frames.clone(),
            tex_size: sheet.tex_size,
            graph,
            state: AnimGraphState::new(entry_animation),
        }
    }
}

pub struct SpriteGraphAnimationSystem<S, R> {
    marker: PhantomData<fn() -> (S, R)>,
}

impl<S, R> SpriteGraphAnimationSystem<S, R>
where
    R: AnimTransitionRule<S>,
{
    pub fn new() -> Self {
        SpriteGraphAnimationSystem {
            marker: PhantomData,
        }
    }
}

impl<S, R> System for SpriteGraphAnimationSystem<S, R>
where
    S: Send + Sync + 'static,
    R: AnimTransitionRule<S> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "SpriteGraphAnimationSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) {
        for (_, (state, anim, sprite)) in cx
            .world
            .query_mut::<(&S, &mut SpriteGraphAnimation<R>, &mut Sprite)>()
        {
            let result = anim.state.animate(state, &anim.graph, cx.clock.delta);
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
                .unwrap_or(frames.last().unwrap());

            sprite.src = Rect {
                left: (frame.src.x as f32) / frame.src_size.w as f32,
                right: (frame.src.x as f32 + frame.src.w as f32) / frame.src_size.w as f32,
                top: (frame.src.y as f32) / frame.src_size.h as f32,
                bottom: (frame.src.y as f32 + frame.src.h as f32) / frame.src_size.h as f32,
            };
            sprite.tex = Rect {
                left: (frame.tex.x as f32) / anim.tex_size.w as f32,
                right: (frame.tex.x as f32 + frame.tex.w as f32) / anim.tex_size.w as f32,
                top: (frame.tex.y as f32) / anim.tex_size.h as f32,
                bottom: (frame.tex.y as f32 + frame.tex.h as f32) / anim.tex_size.h as f32,
            };
        }
    }
}
