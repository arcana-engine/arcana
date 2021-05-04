// #![deny(missing_docs)]

//!
//! arcana crate.
//!

pub mod assets;
pub mod camera;
pub mod event;
pub mod graphics;

mod bitset;
mod clocks;
mod control;
mod debug;
mod funnel;
mod game;
mod physics2;
mod prefab;
mod resources;
mod scene;
mod system;
mod viewport;

pub use self::{
    clocks::{ClockIndex, Clocks, FixedClockStepIter},
    control::{AssumeControlError, Control, ControlResult, InputController},
    debug::{DebugInfo, EntityDebugInfo, EntityDispay, EntityRefDebugInfo, EntityRefDisplay},
    funnel::Funnel,
    game::{game2, game3, Game},
    physics2::{Physics2, PhysicsData2},
    prefab::{prefab_pipe, Prefab, PrefabLoader, PrefabSpawner},
    resources::Res,
    scene::{Global2, Global3, Local2, Local3, SceneSystem},
    system::{Scheduler, System, SystemContext},
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
