// virtualization/src/memory.rs
use crate::Result;
use crate::error::VmmError;
use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::VmFd;
use vm_memory::{GuestAddress, GuestMemoryBackend, GuestMemoryMmap};

/// Guest physical address constants
pub mod guest_address {
    pub const KERNEL_START: u64 = 0x100000; // 1 MB
    pub const BOOT_PARAMS: u64 = 0x90000; // 576 KB
    pub const CMDLINE: u64 = 0x20000; // 128 KB
    pub const INITRAMFS: u64 = 0x1000000; // 16 MB
    pub const HIGH_MEM_START: u64 = 0x0;
}

/// Manages guest memory allocation and registration
pub struct GuestMemoryManager {
    memory: GuestMemoryMmap,
}

impl GuestMemoryManager {
    /// Allocate guest memory of the given size
    pub fn new(size_mb: u64) -> Result<Self> {
        let size_bytes = (size_mb as usize) << 20;
        let memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), size_bytes)])
            .map_err(VmmError::from)?;

        Ok(Self { memory })
    }

    /// Get a reference to the guest memory
    pub fn memory(&self) -> &GuestMemoryMmap {
        &self.memory
    }

    /// Register guest memory with the VM
    pub fn register_with_vm(&self, vm_fd: &VmFd) -> Result<()> {
        let region = kvm_userspace_memory_region {
            slot: 0,
            guest_phys_addr: 0,
            memory_size: self.memory.num_regions() as u64,
            userspace_addr: self.memory.get_host_address(GuestAddress(0)).unwrap() as u64,
            flags: 0,
        };

        unsafe {
            vm_fd
                .set_user_memory_region(region)
                .map_err(VmmError::MemoryRegion)?;
        }

        Ok(())
    }
}
