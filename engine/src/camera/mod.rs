//! Provides types and functions to deal with various types of cameras.

cfg_if::cfg_if! {
    if #[cfg(feature = "2d")] {
        mod d2;
        pub use self::d2::*;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "3d")] {
        mod d3;
        pub use self::d3::*;
    }
}
