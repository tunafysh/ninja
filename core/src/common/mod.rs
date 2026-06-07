//! Common types, configuration, and error handling.
//!
//! This module provides core data structures and functionality used across the Ninja framework:
//!
//! - [`config`]: Global Ninja configuration including registries and settings
//! - [`types`]: Core types like `ShurikenState`, `FieldValue`, and `PlatformPath`
//! - [`error`]: Error types for Shuriken-specific failures
//! - [`registry`]: Registry support for discovering and managing Shurikens

pub mod config;
pub mod error;
pub mod registry;
pub mod structs;
pub mod traits;
pub mod types;
