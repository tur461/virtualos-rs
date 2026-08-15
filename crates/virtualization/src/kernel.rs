use crate::Result;
use crate::error::VmmError;
use crate::memory::guest_address;
use linux_loader::{
    configurator::BootParams,
    loader::{KernelLoader, KernelLoaderResult, bzimage::BzImage},
};
use std::fs::File;
use std::io::Read;
use vm_memory::{Address, Bytes, GuestAddress, GuestMemoryMmap};

/// Loads the kernel and initramfs into guest memory
pub struct VMKernelLoader;

impl VMKernelLoader {
    /// Load bzImage kernel into guest memory
    pub fn load_kernel(
        kernel_path: &std::path::Path,
        guest_memory: &GuestMemoryMmap,
    ) -> Result<KernelLoaderResult> {
        // Read kernel image
        let mut file = File::open(kernel_path).map_err(VmmError::KernelRead)?;
        let mut kernel_bytes = Vec::new();
        file.read_to_end(&mut kernel_bytes)
            .map_err(VmmError::KernelRead)?;

        // Load the kernel
        let kernel_offset = GuestAddress(guest_address::KERNEL_START);
        let highmem_start = GuestAddress(guest_address::HIGH_MEM_START);
        let mut kernel_image = std::io::Cursor::new(&kernel_bytes);

        let load_result = BzImage::load(
            guest_memory,
            Some(kernel_offset),
            &mut kernel_image,
            Some(highmem_start),
        )
        .map_err(VmmError::from)?;

        Ok(load_result)
    }

    /// Load initramfs into guest memory and update boot parameters
    pub fn load_initramfs(
        initramfs_path: &std::path::Path,
        guest_memory: &GuestMemoryMmap,
        boot_params: &mut BootParams,
    ) -> Result<()> {
        let mut file = File::open(initramfs_path).map_err(VmmError::InitramfsRead)?;
        let mut initramfs_bytes = Vec::new();
        file.read_to_end(&mut initramfs_bytes)
            .map_err(VmmError::InitramfsRead)?;

        let initrd_addr = GuestAddress(guest_address::INITRAMFS);
        guest_memory
            .write_slice(&initramfs_bytes, initrd_addr)
            .map_err(VmmError::from)?;

        let ramdisk_image_offset = 0x218; // ramdisk_image offset in setup_header for x86_64
        let ramdisk_size_offset = 0x220; // ramdisk_size offset

        // Ensure the header is large enough
        if boot_params.header.len() > ramdisk_image_offset + 4 {
            // Write ramdisk_image (u32)
            let ramdisk_image_bytes = (initrd_addr.raw_value() as u32).to_le_bytes();
            boot_params.header[ramdisk_image_offset..ramdisk_image_offset + 4]
                .copy_from_slice(&ramdisk_image_bytes);

            // Write ramdisk_size (u32)
            let ramdisk_size_bytes = (initramfs_bytes.len() as u32).to_le_bytes();
            boot_params.header[ramdisk_size_offset..ramdisk_size_offset + 4]
                .copy_from_slice(&ramdisk_size_bytes);
        }
        Ok(())
    }
}
