// #![deny(missing_docs)]
#![feature(allocator_api)]

//!
//! Arcana is a game engine built with focus on ease of use without compromising on level of control.
//!
//! # Getting started
//!
//! Starting writing a game is as simple as calling single function: `arcana::game2` or `arcana::game3`,\
//! depending on what number of dimensions new game needs.\
//! From there add systems, load assets or otherwise populate game world.
//!

pub mod anim;
pub mod assets;
mod bitset;
mod clocks;
mod debug;
mod game;
pub mod lifespan;
pub mod net;
pub mod prefab;

#[cfg(feature = "physics2d")]
pub mod physics2;

#[cfg(feature = "physics3d")]
pub mod physics3;

mod control;
mod resources;
mod scene;
mod scoped_vec_iter;
mod system;
mod task;

#[cfg(feature = "visible")]
pub mod camera;
#[cfg(feature = "visible")]
pub mod event;
#[cfg(feature = "visible")]
pub mod fps;
#[cfg(feature = "visible")]
mod funnel;
#[cfg(feature = "visible")]
pub mod graphics;
#[cfg(feature = "visible")]
mod viewport;

pub use {hecs, na, scoped_arena};

#[cfg(feature = "visible")]
pub use sierra;

pub use arcana_proc::timespan;

pub use self::{
    clocks::{ClockIndex, Clocks, TimeSpan, TimeSpanParseErr},
    control::CommandQueue,
    debug::{DebugInfo, EntityDebugInfo, EntityDisplay, EntityRefDebugInfo, EntityRefDisplay},
    game::*,
    resources::Res,
    scene::*,
    system::{Scheduler, System, SystemContext},
    task::{AsyncTaskContext, Spawner, TaskContext},
};

#[cfg(feature = "visible")]
pub use self::{
    control::{
        AssumeControlError, Control, ControlResult, Controlled, EntityController, InputCommander,
        InputController, InputEvent,
    },
    funnel::Funnel,
    graphics::renderer::{self, Renderer},
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
