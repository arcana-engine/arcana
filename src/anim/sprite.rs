use {
    super::graph::{
        AnimGraph, AnimGraphState, AnimNode, AnimTransitionRule, CurrentAnimInfo, Transition,
    },
    crate::{
        assets::{SpriteFrame, SpriteSheet, SpriteSize},
        graphics::{Rect, Sprite},
        system::{System, SystemContext},
    },
    std::{marker::PhantomData, sync::Arc},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FaceDirection {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Character2DAnimState {
    pub face: FaceDirection,
    pub running: bool,
    pub jumping: bool,
    pub airborne: bool,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]

pub enum Character2DAnimRule {
    /// Matches state if not running, not jumping and not falling.
    IsIdle { face: FaceDirection },

    /// Matches state if grounded and running and face direction is same as specified.
    IsRunning { face: FaceDirection },

    /// Matches state if jumping and face direction is same as specified.
    IsJumping { face: FaceDirection },

    /// Matches state if not grounded and face direction is same as specified.
    IsAirborne { face: FaceDirection },

    /// Matches state if grounded and face direction is same as specified.
    IsNotAirborne { face: FaceDirection },

    /// Matches if animation ended and face direction is same as specified.
    AnimationEnded { face: FaceDirection },
}

impl AnimTransitionRule<Character2DAnimState> for Character2DAnimRule {
    fn matches(&self, state: &Character2DAnimState, info: &CurrentAnimInfo) -> bool {
        match self {
            Self::IsIdle { face } => {
                !state.running && !state.jumping && !state.airborne && state.face == *face
            }
            Self::IsRunning { face } => {
                state.running && !state.jumping && !state.airborne && state.face == *face
            }
            Self::IsJumping { face } => state.jumping && state.face == *face,
            Self::IsAirborne { face } => !state.jumping && state.airborne && state.face == *face,
            Self::IsNotAirborne { face } => !state.airborne && state.face == *face,
            Self::AnimationEnded { face } => info.span == info.elapsed && state.face == *face,
        }
    }
}

pub const SIMPLE_GRAPH: &'static str = r#"
{
    "animations": [
        {
            "animation": {"name": "idle_left", "from": 0, "to": 1},
            "span": "500ms",
            "transitions": [2, 3, 4, 5, 6, 7, 8, 1]
        },
        {
            "animation": {"name": "idle_right", "from": 2, "to": 3},
            "span": "500ms",
            "transitions": [2, 3, 4, 5, 6, 7, 9, 0]
        },
        {
            "animation": {"name": "run_left", "from": 4, "to": 7},
            "span": "1000ms",
            "transitions": [0, 1, 2, 3, 4, 5, 7, 10]
        },
        {
            "animation": {"name": "run_right", "from": 8, "to": 11},
            "span": "1000ms",
            "transitions": [0, 1, 2, 3, 4, 5, 8, 11]
        },
        {
            "animation": {"name": "jump_left", "from": 12, "to": 12},
            "span": "500ms",
            "transitions": [12, 13, 14, 15]
        },
        {
            "animation": {"name": "jump_right", "from": 15, "to": 15},
            "span": "500ms",
            "transitions": [12, 13, 14, 15]
        },
        {
            "animation": {"name": "fall_left", "from": 16, "to": 16},
            "span": "500ms",
            "transitions": [14, 15]
        },
        {
            "animation": {"name": "fall_right", "from": 17, "to": 17},
            "span": "500ms",
            "transitions": [14, 15]
        },
        {
            "animation": {"name": "land_left", "from": 18, "to": 18},
            "span": "200ms",
            "transitions": [9, 10]
        },
        {
            "animation": {"name": "land_right", "from": 19, "to": 19},
            "span": "200ms",
            "transitions": [9, 10]
        }
    ],
    "transitions": [
        {
            "rule": { "IsIdle": { "face": "Left" } },
            "target": 0
        },
        {
            "rule": { "IsIdle": { "face": "Right" } },
            "target": 1
        },
        {
            "rule": { "IsJumping": { "face": "Left" } },
            "target": 4
        },
        {
            "rule": { "IsJumping": { "face": "Right" } },
            "target": 5
        },
        {
            "rule": { "IsAirborne": { "face": "Left" } },
            "target": 6
        },
        {
            "rule": { "IsAirborne": { "face": "Right" } },
            "target": 7
        },
        {
            "rule": { "IsRunning": { "face": "Left" } },
            "target": 2
        },
        {
            "rule": { "IsRunning": { "face": "Right" } },
            "target": 3
        },
        {
            "rule": { "AnimationEnded": { "face": "Left" } },
            "target": 0
        },
        {
            "rule": { "AnimationEnded": { "face": "Right" } },
            "target": 1
        },
        {
            "rule": { "AnimationEnded": { "face": "Left" } },
            "target": 2
        },
        {
            "rule": { "AnimationEnded": { "face": "Right" } },
            "target": 3
        },
        {
            "rule": { "AnimationEnded": { "face": "Left" } },
            "target": 6
        },
        {
            "rule": { "AnimationEnded": { "face": "Right" } },
            "target": 7
        },
        {
            "rule": { "IsNotAirborne": { "face": "Left" } },
            "target": 8
        },
        {
            "rule": { "IsNotAirborne": { "face": "Right" } },
            "target": 9
        }
    ]
}
"#;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FrameSpan {
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

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
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

        Ok(())
    }
}
