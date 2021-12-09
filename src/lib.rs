// #![deny(missing_docs)]
#![feature(allocator_api, hash_drain_filter, ready_macro)]

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
pub mod camera;
pub mod clocks;
pub mod command;
pub mod debug;
pub mod fps;
pub mod game;
pub mod lifespan;
pub mod prefab;
pub mod rect;
pub mod resources;
pub mod system;
pub mod task;
pub mod ui;

cfg_if::cfg_if! {
    if #[cfg(feature = "visible")] {
        pub mod event;
        pub mod graphics;
        pub use sierra;
        pub mod control;
        pub mod viewport;
        pub mod funnel;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "2d")] {
        pub mod tiles;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "physics2d")] {
        pub mod physics2;
        pub use {rapier2d, parry2d};
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "physics3d")] {
        pub mod physics3;
        pub use {rapier3d, parry3d};
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "sigils")] {
        pub use sigils;
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(feature = "2d", feature = "3d"))] {
        pub mod scene;
    }
}

// Reexport crates used in public API.
pub use {bincode, evoke, hecs, na, palette, scoped_arena};

// Reexport proc-macros
pub use arcana_proc::timespan;

pub mod prelude;

/// Installs default eyre handler.
pub fn install_eyre_handler() {
    if let Err(err) = color_eyre::install() {
        panic!("Failed to install eyre report handler: {}", err);
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
        panic!("Failed to install tracing subscriber: {}", err);
    }
}
