//! # Ninja Core Library
//!
//! The core library for Ninja, a service orchestration and management framework.
//! This library provides the foundational components for managing Shurikens
//! (services) including configuration, scripting, installation, and lifecycle management.
//!
//! ## Main Components
//!
//! - [`manager`]: Main orchestrator for Shuriken management (`ShurikenManager`)
//! - [`shuriken`]: Shuriken service representation and operations
//! - [`common`]: Core types, configuration, and error handling
//! - [`scripting`]: Lua-based DSL and script execution engine
//! - [`utils`]: Utility functions for file operations, downloads, and system integration
//! - [`backup`]: Backup and restore functionality
//!
//! ## Quick Start
//!
//! ```ignore
//! use ninja_core::manager::ShurikenManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize the manager
//!     let manager = ShurikenManager::new().await?;
//!
//!     // List all available Shurikens
//!     let shurikens = manager.list(false).await?;
//!
//!     // Start a specific Shuriken
//!     manager.start(\"my-service\").await?;
//!
//!     // Configure it
//!     manager.configure(\"my-service\").await?;
//!
//!     Ok(())
//! }
//! ```

/// Backup and restore functionality for Shurikens
pub mod backup;

/// Core types, configuration, and error definitions\npub mod common;
pub mod common;

/// Main orchestrator for managing Shuriken services\npub mod manager;
pub mod manager;

/// Lua scripting engine and DSL support\npub mod scripting;
pub mod scripting;

/// Individual Shuriken service representation and operations\npub mod shuriken;
pub mod shuriken;

/// Utility functions for common operations\npub mod utils;
pub mod utils;

/// Version string from Cargo.toml\npub const VERSION: &str = env!(\"CARGO_PKG_VERSION\");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
