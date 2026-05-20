#![feature(custom_test_frameworks)] // replace Rust's default test runner
#![test_runner(rusty_os::test_runner)] // Specifies the function `test_runner` to run test
#![reexport_test_harness_main = "test_main"]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use rusty_os::println;

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

#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    println!("Hello world");

    rusty_os::init();

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3();

    // fn stack_overflow() {
    //     stack_overflow();
    // }

    // stack_overflow();
    //
    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at : {:?}",
        level_4_page_table.start_address()
    );

    #[cfg(test)]
    test_main();

    println!("It didn't crash!");
    rusty_os::hlt_loop();
}
