
(May 10 2026)

---

Today is the fourth day of my `"Writing an OS from scratch in Rust#` journey - `rust_os`.

I followed the `Testing` post of `Philipp Oppermann's blog`. I learned so many things today.

Here's the breakdown

---
### Testing

Today I explored unit and integration testing in `no_std` executables.
Testing in `no_std` environment is pretty complicated since `test` crate depends on the standard library which is not available for my bare metal target.

### Custom Test Framework

Fortunately, Rust supports replacing the default test framework through the unstable [custom_test_frameworks](https://doc.rust-lang.org/unstable-book/language-features/custom-test-frameworks.html) features.
This feature requires no external libraries and thus also works in `#[no_std]` environments.

I also learned more about this [`inner attribute`](rust_topics/records/inner_attribute). 

Then I run `cargo test` 
![[Screenshot From 2026-05-09 16-46-30 1.png]]
It still prints "Hello world" instead of the message from my `test_runner`.
The reason was that my `_start` function is still used as entry point.
The `custom_test_frameworks` features generates a `main` function that calls `test_runner`, but this function is ignored because I used the `#[no_main]` attribute and provide my own entry point.


To fix this, I first needed to change the name of the generated function to something different than `main` through the `reexport_test_harness_main` attribute. Then I can call the renamed function from our `_start` function.

Then I create my first test function `trivial_assertion()` and executed `cargo test`.

![[Screenshot From 2026-05-10 08-46-58.png]]

---
### Exiting the QEMU

I needed to manually quit `quemu` on each `cargo test` which is little inconvenient. 
`QEMU` supports a special `isa-debug-exit` device, which provides an easy way to exit `QEMU` from the guest system. To enable it, I needed to pass a `-device` argument to `QEMU`.

I also needed to learn little bit theory on [`I/O ports`](topics/io_ports).

Then I created `QemuExitCode` struct and `exit_qemu()` function which helped to exit the `QEMU` with success or failure code.

---

### Printing to the Console

While running `cargo test`, the `QEMU` immediately exits So I can't see the output of the test.

So I needed a way to print the test output on the console. 
There are various ways to achieve this, for e.g sending the data over a TCP network interface. However, setting up a networking stack is quite a complex task.

### Serial Port

There is a simple way to send the data which is to use [`serial port`](topics/serial_port).

So I added `uart_16550` dependencies and wrote some code which prints on the console.

```toml
[dependencies]
uart_16550 = "0.6.0"
```

```rust
mod serial;
```

```rust
// in src/serial.rs

use uart_16550::{Config, Uart16550Tty, backend::PioBackend};
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SERIAL1: Mutex<Uart16550Tty<PioBackend>> = Mutex::new(unsafe {
        Uart16550Tty::new_port(0x3F8, Config::default())
            .expect("failed to initialize UART")
    });
}
```

```rust
// in src/serial.rs

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
```


Now I can print to the serial interface instead of the `VGA` text buffer in my test code:

```rust
// in src/main.rs

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    […]
}

#[test_case]
fn trivial_assertion() {
    serial_print!("trivial assertion... ");
    assert_eq!(1, 1);
    serial_println!("[ok]");
}
```

#### `QEMU` Arguments

I needed to use `-serial` argument to redirect the output to stdout:

```toml
# in Cargo.toml

[package.metadata.bootimage]
test-args = [
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-serial", "stdio"
]
```

Now I ran `cargo test`:

![[Screenshot From 2026-05-10 10-56-02.png]]

However, when the test fails, the output prints inside `QEMU` because our panic handler still uses `println`.

![[Screenshot From 2026-05-10 10-57-04.png]]
#### Print an Error Message on Panic

To exit QEMU with an error message on a panic, I used [conditional compilation](https://doc.rust-lang.org/1.30.0/book/first-edition/conditional-compilation.html) to use a different panic handler in testing mode:

```rust
// in src/main.rs

// our existing panic handler
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

// our panic handler in test mode
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}
```


For my test panic handler, I used `serial_println` instead of `println` and then exit `QEMU` with a failure exit code. Note that I still need an endless `loop` after the `exit_qemu` call because the compiler does not know that the `isa-debug-exit` device causes a program exit.

Now `QEMU` also exits for failed tests and prints a useful error message on the console:

![[Screenshot From 2026-05-10 11-01-25.png]]


---

That's it for today. I learned so many things like `inner attribute`, `mmio vs io`, `data passed through serial port`, `rust macros` etc.
Peace :).
