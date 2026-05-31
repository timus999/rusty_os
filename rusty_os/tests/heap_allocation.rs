#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rusty_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::allocator;
    use rusty_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    rusty_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // let mut frame_allocator = memory::EmptyFrameAllocator;
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rusty_os::test_panic_handler(info)
}

use alloc::boxed::Box;

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(4);
    let heap_value_2 = Box::new(1);
    assert_eq!(*heap_value_1, 4);
    assert_eq!(*heap_value_2, 1);
}

use alloc::vec::Vec;
#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }

    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

use rusty_os::allocator::HEAP_SIZE;
#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn string_alloc() {
    let s = alloc::string::String::from("hello");
    assert_eq!(s, alloc::string::String::from("hello"));
}

pub struct MyStruct {
    input: String,
}

use alloc::string::String;
impl MyStruct {
    pub fn new() -> Self {
        MyStruct {
            input: String::new(),
        }
    }
}

#[test_case]
fn struct_alloc() {
    let s = MyStruct::new();
    assert!(true);
}

#[test_case]
fn string_op() {
    let mut s = String::new();
    s.push('s');
    assert_eq!(String::from('s'), s);
}
