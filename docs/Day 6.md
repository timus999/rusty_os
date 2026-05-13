
(May 12 2026)

---

This is the Day 6 in my `"Writing my own OS from scratch in rust` journey - `rusty_os`.
Today I worked more on implementing testing environment for my kernel.
I did some refactoring and some in the project.
Here's the breakdown:

---

### Hiding Qemu

Since I report out the complete test results using the `isa-debug-exit` device and the serial port, I don’t need the QEMU window anymore. I hide it by passing the `-display none` argument to QEMU:
```toml
[package.metadata.bootimage]
test-args = [
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-serial", "stdio",
    "-display", "none"
]
```

---
### Timeouts

Since `cargo test` waits until the test runner exits, a test that never returns can block the test runner forever.
So to avoid endless loop I set timeout to be 120 seconds:

```toml
[package.metadata.bootimage]
test-timeout = 120          # (in seconds)
```

---
### Insert Printing Automatically

 The test function currently needs to print its own status information using `serial_print!`/`serial_println!`.
 
Manually adding these print statements for every test I write was cumbersome, so I updated my `test_runner` to print these messages automatically. To do that, I created a new `Testable` trait:

```rust
// in src/main.rs

pub trait Testable {
    fn run(&self) -> ();
}
```

I implemented the `run` function by first printing the function name using the [`any::type_name`](https://doc.rust-lang.org/stable/core/any/fn.type_name.html) function. This function is implemented directly in the compiler and returns a string description of every type. For functions, the type is their name, so this is exactly what I wanted in this case. The `\t` character is the [tab character](https://en.wikipedia.org/wiki/Tab_character), which adds some alignment to the `[ok]` messages.

```rust
// in src/main.rs

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}
```

After printing the function name, I invoked the test function through `self()`. This only works because I required that `self` implements the `Fn()` trait. After the test function returns, I print `[ok]` to indicate that the function did not panic.

And then I updated `test_runner` to use the new `Testable` trait:

```rust
// in src/main.rs

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) { // new
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run(); // new
    }
    exit_qemu(QemuExitCode::Success);
}
```

I removed the print statements from `trivial_assertion` test since they’re now printed automatically:

```rust
// in src/main.rs

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
```

The `cargo test` output was like this:

```
Running 1 tests
rusty_os::trivial_assertion...	[ok]
```

---

### Testing the `VGA` Buffer

I created a few tests for my VGA buffer implementation. First, I created a very simple test to verify that `println` works without panicking:

```rust
// in src/vga_buffer.rs

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}
```

To ensure that no panic occurred even if many lines are printed and lines are shifted off the screen, I created another test:
```rust
// in src/vga_buffer.rs

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}
```

I also created a test function to verify that the printed lines really appeared on the screen:

```rust
// in src/vga_buffer.rs

#[test_case]
fn test_println_output() {
    let s = "Some test string that fits on single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        unsafe {
            let buffer_ptr = WRITER.lock().buffer.as_raw_ptr().as_ptr();
            let row_ptr = addr_of_mut!((*buffer_ptr).chars[BUFFER_HEIGHT - 2]);
            let char_ptr = addr_of_mut!((*row_ptr)[i]);
            assert_eq!(char::from((*char_ptr).ascii_character), c);
        }
    }
}

```

---
## Integration Tests

The convention for [integration tests](https://doc.rust-lang.org/book/ch11-03-test-organization.html#integration-tests) in Rust is to put them into a `tests` directory in the project root (i.e., next to the `src` directory). Both the default test framework and custom test frameworks will automatically pick up and execute all tests in that directory.

All integration tests are their own executables and completely separate from `main.rs`. This means that each test needs to define its own entry point function. 

I created an example integration test named `basic_boot` to see how it worked in detail:

```rust
// in tests/basic_boot.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

fn test_runner(tests: &[&dyn Fn()]) {
    unimplemented!();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}
```

Since integration tests are separate executables, I needed to provide all the crate attributes (`no_std`, `no_main`, `test_runner`, etc.) again. I also created a new entry point function `_start`, which calls the test entry point function `test_main`. I don’t need any `cfg(test)` attributes because integration test executables are never built in non-test mode.

---
### Library

To make the required functions available to integration test, I split off a library from `main.rs`, which can be included by other crates and integration test executables. To do this, I created a new `src/lib.rs` file:

```rust
// src/lib.rs

#![no_std]
```

Like the `main.rs`, the `lib.rs` is a special file that is automatically recognized by cargo. The library is a separate compilation unit, so I needed to specify the `#![no_std]` attribute again.

To make the library work with `cargo test`, I also moved the test functions and attributes from `main.rs` to `lib.rs`:

```rust
// in src/lib.rs

#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

/// Entry point for `cargo test`
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
```

To make `test_runner` available to executables and integration tests, I made it public and didn’t apply the `cfg(test)` attribute to it. I also factored out the implementation of the panic handler into a public `test_panic_handler` function, so that it is available for executables too.

Since the `lib.rs` is tested independently of the `main.rs`, I added a `_start` entry point and a panic handler when the library is compiled in test mode. By using the [`cfg_attr`](https://doc.rust-lang.org/reference/conditional-compilation.html#the-cfg_attr-attribute) crate attribute, I conditionally enabled the `no_main` attribute in this case.

I also moved over the `QemuExitCode` enum and the `exit_qemu` function to  `lib.rs` and made them public.

To also make `println` and `serial_println` available, I moved the module declarations too:

```rust
// in src/lib.rs

pub mod serial;
pub mod vga_buffer;
```


I made the modules public to make them usable outside of the library. This was also required for making the `println` and `serial_println` macros usable since they use the `_print` functions of the modules.

Then I updated the `main.rs` to use the library:

```rust
// in src/main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rusty_os::println;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    #[cfg(test)]
    test_main();

    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rusty_os::test_panic_handler(info)
}
```

---

### Completing the Integration Test

Like the `src/main.rs`, `tests/basic_boot.rs` executable can import types from the new library. This allowed to import the missing components to complete the test:

```rust
// in tests/basic_boot.rs

#![test_runner(blog_os::test_runner)]

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rusty_os::test_panic_handler(info)
}
```

When I ran `cargo test`, it build and ran the tests for the `lib.rs`, `main.rs`, and `basic_boot.rs` separately after each other. For the `main.rs` and the `basic_boot` integration tests, it reported “Running 0 tests” since these files don’t have any functions annotated with `#[test_case]`.

Then I added tests to `basic_boot.rs`. I tested that `println` worked without panicking, like it did in the VGA buffer tests:

```rust
// in tests/basic_boot.rs

use rusty_os::println;

#[test_case]
fn test_println() {
    println!("test_println output");
}
```

---

### Tests that should Panic

The test framework of the standard library supports a [`#[should_panic]` attribute](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics) that allows constructing tests that should fail. This is useful, for example, to verify that a function fails when an invalid argument is passed. Unfortunately, this attribute isn’t supported in `#[no_std]` crates since it requires support from the standard library.

While I can’t use the `#[should_panic]` attribute in my kernel, I can get similar behavior by creating an integration test that exits with a success error code from the panic handler. 
So I created a test with the name `should_panic`:

```rust
// in tests/should_panic.rs

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use rusty_os::{QemuExitCode, exit_qemu, serial_println};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
```

Then I add `_start` and custom `test_runner` function.

```rust
// in tests/should_panic.rs

#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
        serial_println!("[test did not panic]");
        exit_qemu(QemuExitCode::Failed);
    }
    exit_qemu(QemuExitCode::Success);
}
```

Instead of reusing the `test_runner` from the `lib.rs`, the test defines its own `test_runner` function that exits with a failure exit code when a test returns without panicking (I wanted tests to panic). If no test function is defined, the runner exits with a success error code. Since the runner always exits after running a single test, it does not make sense to define more than one `#[test_case]` function.

Then I created a test that should fail:

```rust
// in tests/should_panic.rs

use rusty_os::serial_print;

#[test_case]
fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}
```

When I ran the test through `cargo test --test should_panic` it was successful because the test panicked as expected. When I commented out the assertion and ran the test again, it indeed failed with the _“test did not panic”_ message.

A significant drawback of this approach was that it only works for a single test function. With multiple `#[test_case]` functions, only the first function is executed because the execution cannot continue after the panic handler has been called. 

---

This was the summary of Day 6. I got to learn so much again. It's fun to build your own OS from scratch and understand everything what's happening.