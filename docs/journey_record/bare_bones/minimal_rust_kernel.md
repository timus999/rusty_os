
(May 8 2026)

---

This is the second day in my "writing my own os in Rust" - rusty_os.

Today, There was so much to learn. As I was going through the https://os.phil-opp.com/minimal-rust-kernel/ blog_02, there was so many topics that i needed to cover.

So I will walk through what I did today and put all the topics that I learned today and what knowledge I gained under that topic.


### The Boot process

On x86, there are two firmware standards:
1. BIOS (Basic Input/Output System)
2. UEFI (Unified Extensible Firmware Interface)

Bios standard is old and outdated, but simple and well-supported on any x86 machine.
Uefi is more modern and has much more features, but is more complex to set up.

So I'm using Bios for now.

### Bios Boot

When you turn on a computer, it loads the BIOS from some special flash memory located on the motherboard. The BIOS runs self-test and initialization routines of the hardware, then it looks for bootable disks. If it finds one, control is transferred to its _bootloader_, which is a 512-byte portion of executable code stored at the disk’s beginning. Most bootloaders are larger than 512 bytes, so bootloaders are commonly split into a small first stage, which fits into 512 bytes, and a second stage, which is subsequently loaded by the first stage.

The bootloader has to determine the location of the kernel image on the disk and load it into memory. It also needs to switch the CPU from the 16-bit [real mode](Modes.md) first to the 32-bit [protected mode](topics/Modes), and then to the 64-bit [long mode](topics/Modes), where 64-bit registers and the complete main memory are available. Its third job is to query certain information (such as a memory map) from the BIOS and pass it to the OS kernel.

Writing a bootloader is a bit cumbersome as it requires assembly language and a lot of non insightful steps like "write this magic value to the this processor register".

Therefore I'm not creating bootloader instead there's a tool [bootimage](topics/bootimage) that automatically prepends a bootloader to our kernel.

### A Minimal Kernel

This is where fun begins. Now that I know how a computer boots, it's time to create our own minimal kernel.

My goal was to create a disk image that prints a "Hello World!" to the screen when booted.
Now I extend the previous binary.

First I needed to install the nightly version of the Rust because I needed some experimental features that are only available on the nightly channel.

I did this by running command:
```bash
rustup override set nightly
```

### Target Specification

The x86_64-unknown-linux-gnu target JSON file looks like this:
```json
{
    "llvm-target": "x86_64-unknown-linux-gnu",
    "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    "arch": "x86_64",
    "target-endian": "little",
    "target-pointer-width": 64,
    "target-c-int-width": 32,
    "os": "linux",
    "executables": true,
    "linker-flavor": "gcc",
    "pre-link-args": ["-m64"],
    "morestack": false
}
```

But for my target system, I needed some special configuration parameters(e.g. no underlying OS). Fortunately, Rust allows us to define our own target through a `JSON` file.

I also targeted x86_64 systems with my kernel, so my target specification looks very similar to the one above.

I started by creating `x86_64-rusty_os.json` file with some common content:

```json
{
    "llvm-target": "x86_64-unknown-none",
    "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    "arch": "x86_64",
    "target-endian": "little",
    "target-pointer-width": 64,
    "target-c-int-width": 32,
    "os": "none",
    "executables": true
}
```

Then I added: 

```json
"linker-flavor": "ld.lld",
"linker": "rust-lld",
```

I used the cross-platform [LLD](https://lld.llvm.org/) linker that is shipped with Rust for linking my kernel.

```json
"panic-strategy": "abort",
```

This setting specifies that the target doesn't support stack unwinding on panic, so instead program should abort directly.

```json
"disable-redzone": true,
```

I'm writing a kernel, so I'll need to handle interrupts at some point. To do that safely, I had to disable a certain stack pointer optimization called the ["red zone"](https://os.phil-opp.com/red-zone/), because it would cause stack corruption otherwise.

```json
"features": "-mmx,-sse,+soft-float",
```

The `features` field enables/disables target features. More on this [SIMD](topics/smid)

```json
"rustc-abi": "x86-softfloat"
```

As I want to use the `soft-float` feature, I also needed to tell the Rust compiler that we want to use the corresponding ABI.

#### Putting it together

```json
{
    "llvm-target": "x86_64-unknown-none",
    "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    "arch": "x86_64",
    "target-endian": "little",
    "target-pointer-width": 64,
    "target-c-int-width": 32,
    "os": "none",
    "executables": true,
    "linker-flavor": "ld.lld",
    "linker": "rust-lld",
    "panic-strategy": "abort",
    "disable-redzone": true,
    "features": "-mmx,-sse,+soft-float",
    "rustc-abi": "x86-softfloat"
}
```

### Building our Kernel

The entry points needs to be called \_start regardless of the host OS.

Building the kernel with my new target `x86_64-rusty_os.json` failed because the custom JSON target specifications are an unstable feature that required explicit opt-in.

#### The json-target-spec Option

So, to enable support for custom JSON target specifications, I needed to create a local cargo configuration file at `.cargo/config.toml` at root directory.

```toml
[unstable]
json-target-spec = true
```

After this, I tried to build again:

```bash
cargo build --target x86_64-blog_os.json

error[E0463]: can't find crate for `core`
```

It still failed but with new error. This error told that the Rust compiler can't find the core library.

The problem is that the core library is distributed together with the Rust compiler as a _precompiled_ library. So it was only valid for supported host triples but not for my custom target.

So if I want to compile code for other targets, I needed to recompile `core` for these targets first.

#### The build-std Option

There's [`build-std` feature](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) which allows to recompile `core` and other standard library crates on demand, instead of using the precompiled versions shipped with the Rust installation.

To use this feature, I needed to add the following in my cargo configuration file:

```toml
[unstable]
json-target-spec = true
build-std = ["core", "compiler_builtins"]
```

After that, I rerun my build command and it compiled :). 

### Memory-Related Intrinsics

When building a custom operating system kernel in Rust with `no-std`, the standard C library ( which provides essential memory functions) is unavailable.

The Rust compiler needs low-level memory functions like `memcpy`, `memset`, and `memcmp`, but they are not  provided by a C library in a bare-metal environment.

So, I used the `compiler-builtins` crate, which already contains safe, tested implementation of these functions.

I added the `compiler-builtins-mem` feature by configuring my `.cargo.config.toml` file:

```toml
[unstable]
json-target-spec = true
build-std-features = ["compiler-builtins-mem"]
build-std = ["core", "compiler_builtins"]
```

#### Set a default target

To avoid passing the `--target` parameter on every invocation of `cargo build`, I overrode the default target:

```toml
[build]
target = "x86_64-rusty_os.json"
```


### Printing to Screen

The easiest way to print text to the screen at this stage was the [VGA text buffer](topics/vga_text_buffer).

The implementation: 

```rust
static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}
```

First, I cast the integer `0xb8000` into a raw pointer. Then I iterated over the bytes of the static `HELLO` byte string. I used the `enumerate` method to additionally get a running variable `i`. In the body of the for loop, I used the `offset` method to write the string byte and the corresponding color byte (`0xb` is a light cyan).

### Running My Kernel

It was time to run it. But I needed to turn my compiled kernel into a bootable disk image by linking it with a bootloader.
Then I can run the disk image in the [QEMU](topics/qemu) virtual machine.
I can also boot it on real hardware using a USB drive but that was unnecessary.

#### Creating a Bootimage

Instead of writing my own `bootloader`, which is a project on it's own, I used the `bootloader` crate.
This crate implements a basic BIOS bootloader without any C dependencies, just Rust and inline assembly. 
To add it: 
```bash
cargo add bootloader
```

The current version didn't compiled so I revert back to older version which was 0.9 and it compiled.


Adding the bootloader as a dependency was not enough to actually create a bootable disk image.
The problem was that I needed to link my kernel with the bootloader after compilation, but cargo has no support for [post-build scripts](https://github.com/rust-lang/cargo/issues/545).

So to solve this problem, There was a tool named `bootimage`, So I installed the tool. More on this [tool](topics/bootimage).

### Booting it in QEMU

I booted it in QEMU by following command:

```bash
qemu-system-x86_64 -drive format=raw,file=target/x86_64-rusty_os/debug/bootimage-rusty_os.bin
```

This opened a separate window which is shown below:

![[Screenshot From 2026-05-08 13-24-41.png]]

#### Using cargo run

To make it easier to run my kernel in QEMU, we can set the `runner` configuration key for cargo:

```toml

[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

The `target.'cfg(target_os = "none")'` table applies to all targets whose target configuration file’s `"os"` field is set to `"none"`. This includes my `x86_64-rusty_os.json` target. The `runner` key specifies the command that should be invoked for `cargo run`. The command is run after a successful build with the executable path passed as the first argument.