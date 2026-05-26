#![feature(custom_test_frameworks)] // replace Rust's default test runner
#![test_runner(rusty_os::test_runner)] // Specifies the function `test_runner` to run test
#![reexport_test_harness_main = "test_main"]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rusty_os::println;

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // use rusty_os::memory;
    // use x86_64::structures::paging::{Page, Translate};
    use rusty_os::allocator;
    use rusty_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    println!("Hello world");
    rusty_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // let mut frame_allocator = memory::EmptyFrameAllocator;
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");

    let heap_value = Box::new(42);
    println!("Heap_value at {:p}", heap_value);

    // create a dynamically size vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }

    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be free when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let closed_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&closed_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now ",
        Rc::strong_count(&closed_reference)
    );

    // map an unused page
    // let page = Page::containing_address(VirtAddr::new(0));
    // memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // Write the string `New!` to the screen through new mapping
    // let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

    // let addresses = [
    //     // the identity-mapped vga buffer page
    //     0xb8000,
    //     // some code page
    //     0x201008,
    //     // some stack page
    //     0x0100_0020_1a10,
    //     // virtual address mapped to physical address 0
    //     boot_info.physical_memory_offset,
    // ];

    // for &address in &addresses {
    //     let virt = VirtAddr::new(address);
    //     // let phys = unsafe { translate_addr(virt, phys_mem_offset) };
    //     let phys = mapper.translate_addr(virt);
    //     println!("{:?} -> {:?}", virt, phys);
    // }
    // let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

    // for (i, entry) in l4_table.iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("L4 Entry {}: {:?}", i, entry);

    //         // get the physical address from the entry and convert it
    //         let phys = entry.frame().unwrap().start_address();
    //         let virt = phys.as_u64() + boot_info.physical_memory_offset;
    //         let ptr = VirtAddr::new(virt).as_mut_ptr();

    //         let l3_table: &PageTable = unsafe { &*ptr };

    //         // print non-empty entries of the level 3 table
    //         for (i, entry) in l3_table.iter().enumerate() {
    //             if !entry.is_unused() {
    //                 println!("L3 Entry {}: {:?}", i, entry);
    //             }
    //         }
    //     }
    // }

    #[cfg(test)]
    test_main();

    println!("It didn't crash!");
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

// #[unsafe(no_mangle)] // don't mangle the name of this function
// pub extern "C" fn _start() -> ! {
//     println!("Hello world");

//     rusty_os::init();

//      invoke a breakpoint exception
//      x86_64::instructions::interrupts::int3();

//      fn stack_overflow() {
//          stack_overflow();
//      }

//      stack_overflow();

//     use x86_64::registers::control::Cr3;

//     let (level_4_page_table, _) = Cr3::read();
//     println!(
//         "Level 4 page table at : {:?}",
//         level_4_page_table.start_address()
//     );

//     #[cfg(test)]
//     test_main();

//     println!("It didn't crash!");
//     rusty_os::hlt_loop();
// }
