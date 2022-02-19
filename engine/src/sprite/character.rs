use crate::sprite::graph::AnimNode;

use super::{
    anim::FaceDirection,
    graph::{AnimTransitionRule, CurrentAnimInfo},
};

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

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Character2DAnimState {
    pub face: FaceDirection,
    pub running: bool,
    pub jumping: bool,
    pub airborne: bool,
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

pub fn simple_character_graph() -> AnimGraph<FrameSpan, Character2DAnimRule> {
    use Character2DAnimRule::*;
    use FaceDirection::*;

    AnimGraph {
        animations: vec![
            AnimNode {
                // idle_left
                animation: FrameSpan { from: 0, to: 1 },
                span: TimeSpan::from_millis(500),
                transitions: vec![2, 3, 4, 5, 6, 7, 8, 1],
            },
            AnimNode {
                // idle_right
                animation: FrameSpan { from: 2, to: 3 },
                span: TimeSpan::from_millis(500),
                transitions: vec![2, 3, 4, 5, 6, 7, 9, 0],
            },
            AnimNode {
                // run_left
                animation: FrameSpan { from: 4, to: 7 },
                span: TimeSpan::from_millis(1000),
                transitions: vec![0, 1, 2, 3, 4, 5, 7, 10],
            },
            AnimNode {
                // run_right
                animation: FrameSpan { from: 8, to: 11 },
                span: TimeSpan::from_millis(1000),
                transitions: vec![0, 1, 2, 3, 4, 5, 8, 11],
            },
            AnimNode {
                // jump_left
                animation: FrameSpan { from: 12, to: 12 },
                span: TimeSpan::from_millis(500),
                transitions: vec![12, 13, 14, 15],
            },
            AnimNode {
                // jump_right
                animation: FrameSpan { from: 15, to: 15 },
                span: TimeSpan::from_millis(500),
                transitions: vec![12, 13, 14, 15],
            },
            AnimNode {
                // fall_left
                animation: FrameSpan { from: 16, to: 16 },
                span: TimeSpan::from_millis(500),
                transitions: vec![14, 15],
            },
            AnimNode {
                // fall_right
                animation: FrameSpan { from: 17, to: 17 },
                span: TimeSpan::from_millis(500),
                transitions: vec![14, 15],
            },
            AnimNode {
                // land_left
                animation: FrameSpan { from: 18, to: 18 },
                span: TimeSpan::from_millis(200),
                transitions: vec![9, 10],
            },
            AnimNode {
                // land_right
                animation: FrameSpan { from: 19, to: 19 },
                span: TimeSpan::from_millis(200),
                transitions: vec![9, 10],
            },
        ],
        transitions: vec![
            Transition {
                transition: (),
                rule: IsIdle { face: Left },
                target: 0,
            },
            Transition {
                transition: (),
                rule: IsIdle { face: Right },
                target: 1,
            },
            Transition {
                transition: (),
                rule: IsJumping { face: Left },
                target: 4,
            },
            Transition {
                transition: (),
                rule: IsJumping { face: Right },
                target: 5,
            },
            Transition {
                transition: (),
                rule: IsAirborne { face: Left },
                target: 6,
            },
            Transition {
                transition: (),
                rule: IsAirborne { face: Right },
                target: 7,
            },
            Transition {
                transition: (),
                rule: IsRunning { face: Left },
                target: 2,
            },
            Transition {
                transition: (),
                rule: IsRunning { face: Right },
                target: 3,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Left },
                target: 0,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Right },
                target: 1,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Left },
                target: 2,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Right },
                target: 3,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Left },
                target: 6,
            },
            Transition {
                transition: (),
                rule: AnimationEnded { face: Right },
                target: 7,
            },
            Transition {
                transition: (),
                rule: IsNotAirborne { face: Left },
                target: 8,
            },
            Transition {
                transition: (),
                rule: IsNotAirborne { face: Right },
                target: 9,
            },
        ],
    }
}
