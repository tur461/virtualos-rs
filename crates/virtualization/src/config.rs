// virtualization/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a microVM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Path to the kernel image (bzImage)
    pub kernel_path: PathBuf,

    /// Path to initramfs (optional, gzipped cpio archive)
    pub initramfs_path: Option<PathBuf>,

    /// Command to execute inside the VM
    pub command: String,

    /// Arguments for the command
    pub args: Vec<String>,

    /// Memory size in MB
    pub memory_mb: u64,

    /// Number of vCPUs (currently only 1 is supported)
    pub vcpu_count: u8,
}

impl VmConfig {
    /// Create a new VM configuration with defaults
    pub fn new(kernel_path: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            kernel_path: kernel_path.into(),
            initramfs_path: None,
            command: command.into(),
            args: Vec::new(),
            memory_mb: 128,
            vcpu_count: 1,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), crate::VmmError> {
        if !self.kernel_path.exists() {
            return Err(crate::VmmError::InvalidConfig(format!(
                "Kernel image not found at {}",
                self.kernel_path.display()
            )));
        }

        if self.memory_mb < 64 {
            return Err(crate::VmmError::InvalidConfig(
                "Memory size must be at least 64 MB".to_string(),
            ));
        }

        if self.vcpu_count != 1 {
            return Err(crate::VmmError::InvalidConfig(
                "Only 1 vCPU is currently supported".to_string(),
            ));
        }

        if let Some(initramfs) = &self.initramfs_path
            && !initramfs.exists()
        {
            return Err(crate::VmmError::InvalidConfig(format!(
                "Initramfs not found at {}",
                initramfs.display()
            )));
        }

        Ok(())
    }

    /// Build the kernel command line
    pub fn build_cmdline(&self) -> String {
        let mut cmdline = format!("console=ttyS0 rdinit=/bin/sh -- \"{}\"", self.command);
        if !self.args.is_empty() {
            cmdline.push(' ');
            cmdline.push_str(&self.args.join(" "));
        }
        cmdline
    }
}
