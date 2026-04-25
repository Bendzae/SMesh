//! Adapters to third-party crates.
//!
//! Currently only [`bevy`] (feature `bevy_adapter`, enabled by default),
//! which provides `From` conversions to `bevy::Mesh`, a debug-draw plugin,
//! and a small selection system for interactive tooling.

#[cfg(feature = "bevy_adapter")]
pub mod bevy;
