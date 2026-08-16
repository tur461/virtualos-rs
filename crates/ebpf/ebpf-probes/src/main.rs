#![no_std]
#![no_main]

mod events;
mod filesystem;
mod maps;
mod networking;
mod process;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
