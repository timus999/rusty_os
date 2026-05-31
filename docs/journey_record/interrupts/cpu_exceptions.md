(May 16, 2026)

---

This is the Day 6 in my `"Writing my own OS from scratch in Rust` journey - `rust_os`.

With all the theory in the past 2 days in mind, I moved to implementation part.

---

### Implementation

First, I create new module called `interrupts` - `src/interrupts` which will contain our interrupt and exception handling implementation.
Then I created an `init_idt` functions that creates a new `InterruptDescriptorTable`:

```rust

// in src/interrupts.rs

use x86_64::structures::idt::InterruptDescriptorTable;

pub fn init_idt() {
    let mut idt = InterruptDescriptorTable::new();
}
```

`Note: I'm using x86_64 crate which implements IDT and required function`

Then I created a simple `breakpoint_handler` function and added it to  IDT:

```rust
// in src/interrupts.rs

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;

pub fn init_idt() {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
```

---

### Loading the IDT

In order for the CPU to use the new interrupt descriptor table, I needed to load it using the [`lidt`](https://www.felixcloutier.com/x86/lgdt:lidt) instruction. The `InterruptDescriptorTable` struct of the `x86_64` crate provides a [`load`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.InterruptDescriptorTable.html#method.load) method for that:

```rust
// in src/interrupts.rs

pub fn init_idt() {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.load();
}
```

The `load` method expects a `&'static self`, that is, a reference valid for the complete runtime of the program. The reason is that the CPU will access this table on every interrupt until we load a different IDT. So using a shorter lifetime than `'static` could lead to use-after-free bugs.

In fact, this is exactly what happens here.THe `idt` is created on the stack, so it is only valid inside the `init` function. Afterwards, the stack memory is reused for other functions, so the CPU would interpret random stack memory as IDT. Luckily, the `InterruptDescriptorTable::load` method encodes this lifetime requirement in its function definition, so that the Rust compiler is able to prevent this possible bug at compile time.

In order to fix this problem, we need to store our `idt` at a place where it has a `'static` lifetime. To achieve this, we could allocate our IDT on the heap using [`Box`](https://doc.rust-lang.org/std/boxed/struct.Box.html) and then convert it to a `'static` reference, but I am writing an OS kernel and thus don’t have a heap (yet).

---
### Lazy Static

Fortunately, the `lazy_static` macro exists. Instead of evaluating a `static` at compile time, the macro performs the initialization when the `static` is referenced the first time. Thus, I can do almost everything in the initialization block and even able to read runtime values.

So I used the `lazy_static!` macro to create the static IDT:

```rust
// in src/interrupts.rs

use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}
```

---

### Running It

 Instead of calling it directly from the `main.rs`, I introduced a general `init` function in the `lib.rs`:

```rust
// in src/lib.rs

pub fn init() {
    interrupts::init_idt();
}
```

With this function, I now have a central place for initialization routines that can be shared between the different `_start` functions in `main.rs`, `lib.rs`, and integration tests.

Then I updated the `_start` function of `main.rs` to call `init` and then triggered a breakpoint exception:

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    blog_os::init(); // new

    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3(); // new

    // as before
    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    loop {}
}
```

I ran it in QEMU (using `cargo run`):

![[Screenshot From 2026-05-16 09-48-09.png]]
It worked! The CPU successfully invoked the breakpoint handler, which prints the message, and then returned back to the `_start` function, where the `It did not crash!` message was printed.

---
### Adding a test

Then I created a test that ensured that the above continues to work. First, I updated the `_start` function to also call `init`:

```rust
// in src/lib.rs

/// Entry point for `cargo test`
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    init();      // new
    test_main();
    loop {}
}
```

Remember, this `_start` function is used when running `cargo test --lib`, since Rust tests the `lib.rs` completely independently of the `main.rs`. I needed to call `init` here to set up an IDT before running the tests.

Then I created a `test_breakpoint_exception` test:

```rust
// in src/interrupts.rs

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
```

The test invokes the `int3` function to trigger a breakpoint exception. By checking that the execution continues afterward, I verifid that the breakpoint handler was working correctly.

I tested this by running `cargo test --lib` and following output shown:

```
rusty_os::interrupts::test_breakpoint_exception...	[ok]
```

---

That's it for today. I'm glad that I finally implemented exception handling but there's a lot more. Learning theory was fun but implementing it is more fun.
