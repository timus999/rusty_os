#![feature(custom_test_frameworks)] // replace Rust's default test runner
#![test_runner(rusty_os::test_runner)] // Specifies the function `test_runner` to run test
#![reexport_test_harness_main = "test_main"]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rusty_os::{print, println};

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
entry_point!(kernel_main);

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    rusty_os::init();

    #[cfg(test)]
    test_main();

    print!("> ");

    rusty_os::hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rusty_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rusty_os::test_panic_handler(info);
    rusty_os::hlt_loop();
}
