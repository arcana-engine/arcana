//! Asset store API

use {std::net::ToSocketAddrs, uuid::Uuid};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AssetRequest {
    uuid: Uuid,
}



pub struct 


pub struct StoreManifest {}

pub struct Store {}

impl Store {
    pub fn new() -> Self {}
}
