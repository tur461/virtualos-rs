// virtualization/src/error.rs
use thiserror::Error;

/// Errors that can occur during VM operations
#[derive(Debug, Error)]
pub enum VmmError {
    #[error("Failed to open /dev/kvm: {0}")]
    KvmOpen(#[source] kvm_ioctls::Error),

    #[error("Failed to create VM: {0}")]
    VmCreate(#[source] kvm_ioctls::Error),

    #[error("Failed to create vCPU: {0}")]
    VcpuCreate(#[source] kvm_ioctls::Error),

    #[error("Failed to allocate guest memory: {0}")]
    MemoryAllocation(#[source] std::io::Error),

    #[error("Failed to set user memory region: {0}")]
    MemoryRegion(#[source] kvm_ioctls::Error),

    #[error("Failed to load kernel: {0}")]
    KernelLoad(String),

    #[error("Failed to read kernel image: {0}")]
    KernelRead(#[source] std::io::Error),

    #[error("Failed to read initramfs: {0}")]
    InitramfsRead(#[source] std::io::Error),

    #[error("Failed to setup boot parameters: {0}")]
    BootParams(String),

    #[error("Failed to set vCPU registers: {0}")]
    RegisterSetup(#[source] kvm_ioctls::Error),

    #[error("Failed to run vCPU: {0}")]
    VcpuRun(#[source] kvm_ioctls::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Guest memory access error: {0}")]
    GuestMemory(String),
}

impl From<linux_loader::loader::Error> for VmmError {
    fn from(e: linux_loader::loader::Error) -> Self {
        VmmError::KernelLoad(e.to_string())
    }
}

impl From<vm_memory::GuestMemoryError> for VmmError {
    fn from(e: vm_memory::GuestMemoryError) -> Self {
        VmmError::GuestMemory(e.to_string())
    }
}

impl From<vm_memory::mmap::FromRangesError> for VmmError {
    fn from(e: vm_memory::mmap::FromRangesError) -> Self {
        VmmError::GuestMemory(e.to_string())
    }
}
