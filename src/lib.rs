// #![deny(missing_docs)]

//!
//! Arcana is a game engine built with focus on ease of use without compromising on level of control.
//!
//! # Getting started
//!
//! Starting writing a game is as simple as calling single function: `arcana::game2` or `arcana::game3`,\
//! depending on what number of dimensions new game needs.\
//! From there add systems, load prefabs or otherwise populate game world.
//!
//! Then start writing prefab implementations and input controls, implement custom rendering logic when required.
//!

pub mod camera;
pub mod event;
pub mod graphics;

pub mod anim;
pub mod assets;
mod bitset;
mod clocks;
mod control;
mod debug;
mod funnel;
mod game;
mod physics2;
mod resources;
mod scene;
mod system;
mod task;
mod viewport;

pub use {bumpalo, hecs, na, sierra};

pub use arcana_proc::timespan;

pub use self::{
    clocks::{/* , FixedClockStepIter*/ ClockIndex, Clocks, TimeSpan, TimeSpanParseErr},
    control::{AssumeControlError, Control, ControlResult, InputController},
    debug::{DebugInfo, EntityDebugInfo, EntityDispay, EntityRefDebugInfo, EntityRefDisplay},
    funnel::Funnel,
    game::{game2, game3, Game},
    physics2::{ContactQueue2, Physics2, PhysicsData2},
    // prefab::{prefab_pipe, Prefab, PrefabLoader, PrefabSpawner},
    resources::Res,
    scene::{Global2, Global3, Local2, Local3, SceneSystem},
    system::{Scheduler, System, SystemContext},
    task::{AsyncTaskContext, Spawner, TaskContext},
    viewport::Viewport,
};

/// Installs default eyre handler.
pub fn install_eyre_handler() {
    if let Err(err) = color_eyre::install() {
        tracing::error!("Failed to install eyre report handler: {}", err);
    }
}

/// Installs default tracing subscriber.
pub fn install_tracing_subscriber() {
    use tracing_subscriber::layer::SubscriberExt as _;
    if let Err(err) = tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish()
            .with(tracing_error::ErrorLayer::default()),
    ) {
        tracing::error!("Failed to install tracing subscriber: {}", err);
    }
}
