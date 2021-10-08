pub use astral::client_server::PlayerId;

pub use self::{mapper::EntityMapper, nid::NetId, serde::ReplicaSerde};

mod mapper;
mod nid;
mod serde;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;
