use crate::clocks::TimeSpan;

/// General purpose animation graph.
/// Runs any kind of animations in agnostic way.
/// Specialized graph based animation systems can be built upon this.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AnimGraph<A, R, T = ()> {
    /// Set of animation nodes of the graph.
    /// Contain animation data and transition rules.
    pub animations: Vec<AnimNode<A>>,

    /// Set of transitions between animation nodes.
    pub transitions: Vec<Transition<R, T>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AnimNode<A> {
    /// Animation data.
    pub animation: A,

    /// Duration of the animation.
    pub span: TimeSpan,

    /// Transitions associated with this node.
    pub transitions: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Transition<R, T = ()> {
    /// Rule to perform this transition.
    pub rule: R,

    /// Target animation node of the transition.
    pub target: usize,

    /// Transition data.
    pub transition: T,
}

mod transition_serde {
    use serde::{de::*, ser::*};

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Full<R, T> {
        pub rule: R,
        pub target: usize,
        pub transition: T,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Partial<R> {
        pub rule: R,
        pub target: usize,
    }

    impl<R, T> Serialize for super::Transition<R, T>
    where
        R: Serialize,
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if std::mem::size_of::<T>() == 0 {
                let mut serializer = serializer.serialize_struct("Transition", 2)?;
                serializer.serialize_field("rule", &self.rule)?;
                serializer.serialize_field("target", &self.target)?;
                serializer.end()
            } else {
                let mut serializer = serializer.serialize_struct("Transition", 2)?;
                serializer.serialize_field("rule", &self.rule)?;
                serializer.serialize_field("target", &self.target)?;
                serializer.serialize_field("transition", &self.transition)?;
                serializer.end()
            }
        }
    }

    impl<'de, R, T> Deserialize<'de> for super::Transition<R, T>
    where
        R: Deserialize<'de>,
        T: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            if std::mem::size_of::<T>() == 0 {
                let partial = Partial::deserialize(deserializer)?;
                Ok(Self {
                    rule: partial.rule,
                    target: partial.target,
                    transition: unsafe {
                        // Safe for ZSTs.
                        std::mem::MaybeUninit::uninit().assume_init()
                    },
                })
            } else {
                let full = Full::deserialize(deserializer)?;
                Ok(Self {
                    rule: full.rule,
                    target: full.target,
                    transition: full.transition,
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct CurrentAnimInfo {
    pub span: TimeSpan,
    pub elapsed: TimeSpan,
}

impl CurrentAnimInfo {
    pub fn is_complete(&self) -> bool {
        self.span == self.elapsed
    }
}

/// Trait for animation transition rules.
pub trait AnimTransitionRule<S> {
    /// Checks if this rules matches state and current animation info.
    fn matches(&self, state: &S, info: &CurrentAnimInfo) -> bool;
}

impl<F, S> AnimTransitionRule<S> for F
where
    F: Fn(&S, &CurrentAnimInfo) -> bool,
{
    fn matches(&self, state: &S, info: &CurrentAnimInfo) -> bool {
        (*self)(state, info)
    }
}

/// State which combined with graph gives animation state machine.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AnimGraphState {
    /// Currently running animation node.
    pub current_animation: usize,

    /// How far into current animation.
    pub current_animation_elapsed: TimeSpan,
}

pub struct AnimateResult<'a, A, T = ()> {
    pub animation: &'a A,
    pub span: TimeSpan,
    pub elapsed: TimeSpan,
    pub transition: Option<&'a T>,
}

impl AnimGraphState {
    /// Returns new animation graph state instance with specified state
    /// and started with `entry_animation`.
    pub fn new(entry_animation: usize) -> Self {
        AnimGraphState {
            current_animation: entry_animation,
            current_animation_elapsed: TimeSpan::ZERO,
        }
    }

    /// Runs animation and transitions.
    ///
    /// Uses current state to decide on transitions according to rules in the graph.
    pub fn animate<'a, S, A, R, T>(
        &mut self,
        state: &S,
        graph: &'a AnimGraph<A, R, T>,
        span: TimeSpan,
    ) -> AnimateResult<'a, A, T>
    where
        R: AnimTransitionRule<S>,
    {
        let mut span = span;
        let mut last_transition = None;

        'l: loop {
            let current_animation = &graph.animations[self.current_animation];
            let time_left = current_animation.span - self.current_animation_elapsed;
            if span > time_left {
                self.current_animation_elapsed = current_animation.span;
                span -= time_left;
            } else {
                self.current_animation_elapsed += span;
                span = TimeSpan::ZERO;
            }

            for &idx in &current_animation.transitions {
                let transition = &graph.transitions[idx];
                let matches = transition.rule.matches(
                    state,
                    &CurrentAnimInfo {
                        span: current_animation.span,
                        elapsed: self.current_animation_elapsed,
                    },
                );

                if matches {
                    self.current_animation = transition.target;
                    self.current_animation_elapsed = TimeSpan::ZERO;
                    last_transition = Some(&transition.transition);
                    continue 'l;
                }
            }

            break;
        }

        let current_animation = &graph.animations[self.current_animation];

        AnimateResult {
            animation: &graph.animations[self.current_animation].animation,
            span: current_animation.span,
            elapsed: self.current_animation_elapsed,
            transition: last_transition,
        }
    }
}
