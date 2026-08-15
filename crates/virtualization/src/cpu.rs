// virtualization/src/cpu.rs
use crate::Result;
use crate::error::VmmError;
use kvm_ioctls::VcpuFd;

/// Configure vCPU for 64-bit long mode operation
pub struct CpuConfigurator;

impl CpuConfigurator {
    /// Setup vCPU registers for kernel boot
    pub fn setup_boot_registers(vcpu_fd: &VcpuFd, entry_point: u64) -> Result<()> {
        // Setup special registers for 64-bit mode
        let mut sregs = vcpu_fd.get_sregs().map_err(VmmError::RegisterSetup)?;
        sregs.cs.base = 0;
        sregs.cs.selector = 0;
        sregs.cs.l = 1; // Long mode
        sregs.cs.db = 0; // 64-bit mode
        vcpu_fd.set_sregs(&sregs).map_err(VmmError::RegisterSetup)?;

        // Setup general purpose registers
        let mut regs = vcpu_fd.get_regs().map_err(VmmError::RegisterSetup)?;
        regs.rip = entry_point;
        regs.rflags = 0x2; // Reserved bit
        regs.rsi = 0; // boot_params pointer (we'll set this if needed)
        vcpu_fd.set_regs(&regs).map_err(VmmError::RegisterSetup)?;

        Ok(())
    }
}
