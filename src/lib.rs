// #![deny(missing_docs)]

//!
//! poc crate.
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
    game::{game, Game},
    prefab::{prefab_pipe, Prefab, PrefabLoader, PrefabSpawner},
    resources::Res,
    scene::{Global3, Local3, SceneSystem},
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
