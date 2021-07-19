#![cfg_attr(not(feature = "std"), no_std)]

//! Contains types for time measurement and ticking.

mod span;
mod stamp;

pub use self::{span::*, stamp::*};
