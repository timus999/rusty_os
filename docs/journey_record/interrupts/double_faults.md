(May 17, 2026)

---

This is Day 11 on my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

Today I explored the double fault exception in detail, which occurs when the CPU fails to invoke an exception handler. By handling this exception, I can avoid fatal _triple faults_ that cause a system reset. To prevent triple faults in all cases, I also set up an _Interrupt Stack Table_ to catch double faults on a separate kernel stack.

---

### Double Fault

In simplified terms, a double fault is a special exception that occurs when the CPU fails to invoke an exception handler. For example, it occurs when a page fault is triggered but there is no page fault handler registered in the [Interrupt Descriptor Table](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-descriptor-table) (IDT). So it’s kind of similar to catch-all blocks in programming languages with exceptions, e.g., `catch(...)` in C++ or `catch(Exception e)` in Java or C#.

A double fault behaves like a normal exception. It has the vector number `8` and we can define a normal handler function for it in the IDT. It is really important to provide a double fault handler, because if a double fault is unhandled, a fatal _triple fault_ occurs. Triple faults can’t be caught, and most hardware reacts with a system reset.

### Triggering a Double Fault

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    blog_os::init();

    // trigger a page fault
    unsafe {
        *(0xdeadbeef as *mut u8) = 42;
    };

    // as before
    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    loop {}
}
```

I used `unsafe` to write to the invalid address `0xdeadbeef`. The virtual address is not mapped to a physical address in the page tables, so a page fault occurs. We haven’t registered a page fault handler in our [IDT](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-descriptor-table), so a [double fault](topics/double_fault) occured.

### A Double Fault Handler

A double fault is a normal exception with an error code, so I specified a handler function similar to the breakpoint handler:

```rust
// in src/interrupts.rs

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler); // new
        idt
    };
}

// new
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
```


### Kernel Stack Overflow

A guard page is a special memory page at the bottom of a stack that makes it possible to detect stack overflows. The page is not mapped to any physical frame, so accessing it causes a page fault instead of silently corrupting other memory. The bootloader sets up a guard page for our kernel stack, so a stack overflow causes a _page fault_.

When a page fault occurs, the CPU looks up the page fault handler in the IDT and tries to push the [interrupt stack frame](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-stack-frame) onto the stack. However, the current stack pointer still points to the non-present guard page. Thus, a second page fault occurs, which causes a double fault (according to the above table).

So the CPU tries to call the _double fault handler_ now. However, on a double fault, the CPU tries to push the exception stack frame, too. The stack pointer still points to the guard page, so a _third_ page fault occurs, which causes a _triple fault_ and a system reboot. So the current double fault handler can’t avoid a triple fault in this case.

I provoked a kernel stack overflow by calling a function that recurses endlessly:

```rust
// in src/main.rs

#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    blog_os::init();

    fn stack_overflow() {
        stack_overflow(); // for each recursion, the return address is pushed
    }

    // trigger a stack overflow
    stack_overflow();

    […] // test_main(), println(…), and loop {}
}
```

When I trid this code in QEMU, the system entered a bootloop again.

### Switching Stacks

The x86_64 architecture is able to switch to a predefined, known-good stack when an exception occurs. This switch happens at hardware level, so it can be performed before the CPU pushes the exception stack frame.

The switching mechanism is implemented as an _Interrupt Stack Table_ (IST). The IST is a table of 7 pointers to known-good stacks. In Rust-like pseudocode:

```rust
struct InterruptStackTable {
    stack_pointers: [Option<StackPointer>; 7],
}
```

For each exception handler, we can choose a stack from the IST through the `stack_pointers` field in the corresponding [IDT entry](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-descriptor-table). For example, the double fault handler could use the first stack in the IST. Then the CPU automatically switches to this stack whenever a double fault occurs. This switch would happen before anything is pushed, preventing the triple fault.

### Creating a TSS

I created a new TSS that contains a separate double fault stack in its interrupt stack table. For that, I needed a TSS struct. Fortunately, the `x86_64` crate already contains a [`TaskStateSegment` struct](https://docs.rs/x86_64/0.14.2/x86_64/structures/tss/struct.TaskStateSegment.html) that I can use.

I created the TSS in a new `gdt` module:

```rust
// in src/lib.rs

pub mod gdt;

// in src/gdt.rs

use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}
```

I used `lazy_static` because Rust’s const evaluator is not yet powerful enough to do this initialization at compile time. I defined that the 0th IST entry is the double fault stack (any other IST index would work too). Then I wrote the top address of a double fault stack to the 0th entry. I wrote the top address because stacks on x86 grow downwards, i.e., from high addresses to low addresses.

### The Global Descriptor Table

The Global Descriptor Table (GDT) is a relic that was used for [memory segmentation](https://en.wikipedia.org/wiki/X86_memory_segmentation) before paging became the de facto standard. However, it is still needed in 64-bit mode for various things, such as kernel/user mode configuration or TSS loading.

The GDT is a structure that contains the _segments_ of the program. It was used on older architectures to isolate programs from each other before paging became the standard. For more information about segmentation, check out the equally named chapter of the free [“Three Easy Pieces” book](http://pages.cs.wisc.edu/~remzi/OSTEP/). While segmentation is no longer supported in 64-bit mode, the GDT still exists. It is mostly used for two things: Switching between kernel space and user space, and loading a TSS structure.

#### Creating a GDT

I created a static `GDT` that includes a segment for the `TSS` static:

```rust
// in src/gdt.rs

use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::tss_segment(&TSS));
        gdt
    };
}
```

As before, I used `lazy_static` again. I created a new GDT with a code segment and a TSS segment.

#### Loading the GDT

To load the GDT, I created a new `gdt::init` function that I called from the `init` function:

```rust
// in src/gdt.rs

pub fn init() {
    GDT.load();
}

// in src/lib.rs

pub fn init() {
    gdt::init();
    interrupts::init_idt();
}
```

The problem is that the GDT segments are not yet active because the segment and TSS registers still contained the values from the old GDT. I also needed to modify the double fault IDT entry so that it uses the new stack.

In summary, I needed to do the following:

1. **Reload code segment register**: I changed the GDT, so I should reload `cs`, the code segment register. This was required since the old segment selector could now point to a different GDT descriptor (e.g., a TSS descriptor).
2. **Load the TSS**: I loaded a GDT that contained a TSS selector, but I still needed to tell the CPU that it should use that TSS.
3. **Update the IDT entry**: As soon as the TSS is loaded, the CPU has access to a valid interrupt stack table (IST). Then I can tell the CPU that it should use the new double fault stack by modifying the double fault IDT entry.

For the first two steps, I needed access to the `code_selector` and `tss_selector` variables in the `gdt::init` function. I achieved this by making them part of the static through a new `Selectors` struct:

```rust
// in src/gdt.rs

use x86_64::structures::gdt::SegmentSelector;

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}
```

Then I used the selectors to reload the `cs` register and load the `TSS`:

```rust
// in src/gdt.rs

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};
    
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
```

I reloaded the code segment register using [`CS::set_reg`](https://docs.rs/x86_64/0.14.5/x86_64/instructions/segmentation/struct.CS.html#method.set_reg) and load the TSS using [`load_tss`](https://docs.rs/x86_64/0.14.2/x86_64/instructions/tables/fn.load_tss.html). The functions are marked as `unsafe`, so I needed an `unsafe` block to invoke them. The reason is that it might be possible to break memory safety by loading invalid selectors.

Now that I have loaded a valid TSS and interrupt stack table, I set the stack index for the double fault handler in the IDT:

```rust
// in src/interrupts.rs

use crate::gdt;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // new
        }

        idt
    };
}
```

The `set_stack_index` method is unsafe because the caller must ensure that the used index is valid and not already used for another exception.

That’s it! Now the CPU should switch to the double fault stack whenever a double fault occurs. Thus, I am able to catch _all_ double faults, including kernel stack overflows:
![[Screenshot From 2026-05-18 10-59-33.png]]

---

### Stack Overflow Test

To test the new `gdt` module and ensure that the double fault handler is correctly called on a stack overflow, I added an integration test. The idea is to provoke a double fault in the test function and verify that the double fault handler is called.

I started with a minimal skeleton:

```rust
// in tests/stack_overflow.rs

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unimplemented!();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rusty_os::test_panic_handler(info)
}
```

Like the `panic_handler` test, the test will run [without a test harness](https://os.phil-opp.com/testing/#no-harness-tests). The reason is that I can’t continue execution after a double fault, so more than one test doesn’t make sense. To disable the test harness for the test, I added the following to the `Cargo.toml`:

```toml
# in Cargo.toml

[[test]]
name = "stack_overflow"
harness = false
```

### Implementing `_start`

The implementation of the `_start` function looks like this:

```rust
// in tests/stack_overflow.rs

use rusty_os::serial_print;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    rusty_os::gdt::init();
    init_test_idt();

    // trigger a stack overflow
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
    unsafe {
        volatile::VolatilePtr::new((&0).into());
    }; // prevent tail recursion optimizations
}
```

I called the `gdt::init` function to initialize a new GDT. Instead of calling the `interrupts::init_idt` function, I called an `init_test_idt` function that will be explained in a moment. The reason is that I want to register a custom double fault handler that does an `exit_qemu(QemuExitCode::Success)` instead of panicking.

The `stack_overflow` function is almost identical to the function in the `main.rs`. The only difference is that at the end of the function, I performed an additional [volatile](https://en.wikipedia.org/wiki/Volatile_\(computer_programming\)) read using the [`Volatile`](https://docs.rs/volatile/0.2.6/volatile/struct.Volatile.html) type to prevent a compiler optimization called [_tail call elimination_](https://en.wikipedia.org/wiki/Tail_call). Among other things, this optimization allows the compiler to transform a function whose last statement is a recursive function call into a normal loop. Thus, no additional stack frame is created for the function call, so the stack usage remains constant.

In my case, however, I wanted the stack overflow to happen, so I added a dummy volatile read statement at the end of the function, which the compiler is not allowed to remove. Thus, the function is no longer _tail recursive_, and the transformation into a loop is prevented. I also added the `allow(unconditional_recursion)` attribute to silence the compiler warning that the function recurses endlessly.

### The Test IDT

As noted above, the test needed its own IDT with a custom double fault handler. The implementation looks like this:

```rust
// in tests/stack_overflow.rs

use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(rusty_os::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}
```

The implementation is very similar to the normal IDT in `interrupts.rs`. Like in the normal IDT, I set a stack index in the IST for the double fault handler in order to switch to a separate stack. The `init_test_idt` function loads the IDT on the CPU through the `load` method.

### The Double Fault Handler

The only missing piece is the double fault handler. It looks like this:

```rust
// in tests/stack_overflow.rs

use blog_os::{exit_qemu, QemuExitCode, serial_println};
use x86_64::structures::idt::InterruptStackFrame;

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
```

When the double fault handler is called, I exit QEMU with a success exit code, which marks the test as passed. Since integration tests are completely separate executables, I need to set the `#![feature(abi_x86_interrupt)]` attribute again at the top of the test file.

Then I ran the test through `cargo test --test stack_overflow`. As expected, I saw the `stack_overflow... [ok]` output in the console. 

---
