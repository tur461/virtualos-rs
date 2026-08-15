use linux_loader::configurator::BootParams;
use vm_memory::{Address, Bytes, GuestAddress, GuestMemoryMmap};

use crate::Result;
use crate::error::VmmError;
use crate::memory::guest_address;

/// Sets up Linux boot parameters for the guest
pub struct BootConfigurator;

impl BootConfigurator {
    /// Write boot parameters and command line to guest memory
    pub fn setup_boot_params(
        guest_memory: &GuestMemoryMmap,
        mut boot_params: BootParams,
        cmdline: &str,
    ) -> Result<()> {
        // Write command line
        let cmdline_bytes = cmdline.as_bytes();
        let cmdline_addr = GuestAddress(guest_address::CMDLINE);
        guest_memory
            .write_slice(cmdline_bytes, cmdline_addr)
            .map_err(VmmError::from)?;

        // Update command line pointers in boot params
        // The header field contains the setup header as a byte array
        // cmd_line_ptr is at offset 0x228 in setup_header (for x86_64)
        let cmd_line_ptr_offset = 0x228;
        let cmdline_size_offset = 0x230;
        // Ensure the header is large enough
        if boot_params.header.len() > cmd_line_ptr_offset + 4 {
            // Write cmd_line_ptr (u32)
            let cmd_line_ptr_bytes = (cmdline_addr.raw_value() as u32).to_le_bytes();
            boot_params.header[cmd_line_ptr_offset..cmd_line_ptr_offset + 4]
                .copy_from_slice(&cmd_line_ptr_bytes);

            // Write cmdline_size (u32)
            let cmdline_size_bytes = (cmdline_bytes.len() as u32).to_le_bytes();
            boot_params.header[cmdline_size_offset..cmdline_size_offset + 4]
                .copy_from_slice(&cmdline_size_bytes);
        }
        // Write the boot params header to guest memory
        let boot_params_addr = GuestAddress(guest_address::BOOT_PARAMS);

        guest_memory
            .write_slice(&boot_params.header, boot_params_addr)
            .map_err(VmmError::from)?;
        Ok(())
    }
}
