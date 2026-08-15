// virtualization/src/lib.rs
//! Lightweight KVM-based microVM implementation
//!
//! This module provides a minimal hypervisor abstraction similar to Firecracker,
//! allowing users to run isolated workloads in a virtual machine.

pub mod boot;
pub mod config;
pub mod cpu;
pub mod device;
pub mod error;
pub mod kernel;
pub mod memory;
pub mod vm;

pub use config::VmConfig;
pub use error::VmmError;
pub use vm::Vm;

/// Result type alias for VM operations
pub type Result<T> = std::result::Result<T, VmmError>;

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
