use crate::Result;
use crate::boot::BootConfigurator;
use crate::config::VmConfig;
use crate::cpu::CpuConfigurator;
use crate::device::{DeviceManager, SerialDevice};
use crate::error::VmmError;
use crate::kernel::VMKernelLoader as KernelLoader;
use crate::memory::GuestMemoryManager;
use kvm_ioctls::{Kvm, VcpuExit, VcpuFd, VmFd};
use linux_loader::configurator::BootParams;
use std::sync::Arc;
use std::sync::mpsc;
use tracing::{debug, error, info};
use vm_memory::{Address, GuestAddress};

/// A lightweight KVM-based virtual machine
pub struct Vm {
    vm_fd: Arc<VmFd>,
    vcpu_fd: VcpuFd,
    _guest_memory: GuestMemoryManager,
    device_manager: DeviceManager,
    serial_rx: Option<mpsc::Receiver<u8>>,
}

impl Vm {
    /// Create a new VM from configuration
    pub fn new(config: VmConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;

        // Open KVM
        let kvm = Kvm::new().map_err(VmmError::KvmOpen)?;
        let vm_fd = kvm.create_vm().map_err(VmmError::VmCreate)?;
        let vm_fd = Arc::new(vm_fd);

        // Allocate and register guest memory
        let guest_memory = GuestMemoryManager::new(config.memory_mb)?;
        guest_memory.register_with_vm(&vm_fd)?;

        // Create vCPU
        let vcpu_fd = vm_fd.create_vcpu(0).map_err(VmmError::VcpuCreate)?;

        // Load kernel
        let load_result = KernelLoader::load_kernel(&config.kernel_path, guest_memory.memory())?;

        // Setup boot parameters
        // Setup boot parameters
        let setup_header = load_result
            .setup_header
            .ok_or_else(|| VmmError::InvalidConfig("No setup header in kernel".to_string()))?;

        // Create BootParams from the setup header
        let mut boot_params = BootParams::new(&setup_header, GuestAddress(0));
        let cmdline = config.build_cmdline();

        // Load initramfs if provided
        if let Some(initramfs_path) = &config.initramfs_path {
            KernelLoader::load_initramfs(initramfs_path, guest_memory.memory(), &mut boot_params)?;
        }

        // Write boot parameters to guest memory
        BootConfigurator::setup_boot_params(guest_memory.memory(), boot_params, &cmdline)?;

        // Setup vCPU registers
        let entry_point = load_result.kernel_load.raw_value();
        CpuConfigurator::setup_boot_registers(&vcpu_fd, entry_point)?;

        // Setup serial device
        let (serial_tx, serial_rx) = mpsc::channel();
        let _serial_device = SerialDevice::new(serial_tx, serial_rx);

        // Fix: create proper serial channels
        let (serial_tx, serial_rx) = mpsc::channel();
        let (_input_tx, input_rx) = mpsc::channel();
        let serial_device = SerialDevice::new(serial_tx, input_rx);
        let device_manager = DeviceManager::new(serial_device);

        let vm = Vm {
            vm_fd,
            vcpu_fd,
            _guest_memory: guest_memory,
            device_manager,
            serial_rx: Some(serial_rx),
        };

        info!(
            "VM created successfully with {} MB memory",
            config.memory_mb
        );
        Ok(vm)
    }

    /// Run the VM until it exits
    pub fn run(&mut self) -> Result<i32> {
        info!("Starting VM execution");

        loop {
            match self.vcpu_fd.run() {
                Ok(exit) => {
                    match exit {
                        VcpuExit::IoOut(port, data) => {
                            if let Err(e) = self.device_manager.handle_io_out(port, data) {
                                debug!("Serial output error: {}", e);
                            }
                        }
                        VcpuExit::IoIn(port, data) => {
                            if let Ok(byte) = self.device_manager.handle_io_in(port) {
                                // Write input byte to vCPU
                                let _ = data;
                                let _ = byte;
                            }
                        }
                        VcpuExit::Hlt => {
                            info!("Guest halted");
                            break;
                        }
                        VcpuExit::Shutdown => {
                            info!("Guest shutdown");
                            break;
                        }
                        VcpuExit::FailEntry(reason, _) => {
                            error!("KVM_RUN failed entry: {:?}", reason);
                            return Err(VmmError::InvalidConfig(format!(
                                "VM entry failed: {:?}",
                                reason
                            )));
                        }
                        other => {
                            debug!("Unhandled VM exit: {:?}", other);
                        }
                    }
                }
                Err(e) => {
                    error!("KVM_RUN error: {}", e);
                    return Err(VmmError::VcpuRun(e));
                }
            }
        }

        Ok(0)
    }

    /// Get a receiver for serial output
    pub fn take_serial_receiver(&mut self) -> Option<mpsc::Receiver<u8>> {
        self.serial_rx.take()
    }

    /// Get VM file descriptor
    pub fn vm_fd(&self) -> &Arc<VmFd> {
        &self.vm_fd
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        info!("VM dropped");
        // KVM file descriptors are automatically cleaned up
    }
}
