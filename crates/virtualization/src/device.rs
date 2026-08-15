// virtualization/src/device.rs
use std::{
    io,
    sync::mpsc::{Receiver, Sender},
};

/// Serial console device (UART 16550)
pub struct SerialDevice {
    tx: Sender<u8>,
    rx: Receiver<u8>,
}

impl SerialDevice {
    /// Create a new serial device with channels
    pub fn new(tx: Sender<u8>, rx: Receiver<u8>) -> Self {
        Self { tx, rx }
    }

    /// Handle output to the serial port
    pub fn handle_output(&self, data: &[u8]) -> io::Result<()> {
        for &byte in data {
            self.tx.send(byte).map_err(|_| {
                io::Error::new(io::ErrorKind::BrokenPipe, "Serial receiver dropped")
            })?;
        }
        Ok(())
    }

    /// Get input from the serial port (currently not implemented for interactive use)
    pub fn handle_input(&self) -> io::Result<u8> {
        self.rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "No data available"))
    }
}

/// Simple device manager for handling I/O ports
pub struct DeviceManager {
    serial: SerialDevice,
}

impl DeviceManager {
    pub fn new(serial: SerialDevice) -> Self {
        Self { serial }
    }

    /// Handle I/O port output
    pub fn handle_io_out(&self, port: u16, data: &[u8]) -> io::Result<()> {
        match port {
            0x3f8 => self.serial.handle_output(data), // COM1
            _ => Ok(()),                              // Ignore other ports
        }
    }

    /// Handle I/O port input
    pub fn handle_io_in(&self, port: u16) -> io::Result<u8> {
        match port {
            0x3f8 => self.serial.handle_input(), // COM1
            _ => Ok(0),                          // Default response for other ports
        }
    }
}
