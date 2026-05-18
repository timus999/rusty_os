(May 18, 2026)

---

This is the Day 12 on my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

Today I learned how to enable and handle external interrupts. I learned about the 8259 PIC and its primary/secondary layout, the remapping of the interrupt numbers, and the “end of interrupt” signal. I implemented handlers for the hardware timer and the keyboard and learned about the `hlt` instruction, which halts the CPU until the next interrupt.

---

### Hardware Interrupts

Interrupts provide a way to notify the CPU from attached hardware devices. So instead of letting the kernel periodically check the keyboard for new characters (a process called [_polling_](https://en.wikipedia.org/wiki/Polling_\(computer_science\))), the keyboard can notify the kernel of each keypress. This is much more efficient because the kernel only needs to act when something happened. It also allows faster reaction times since the kernel can react immediately and not only at the next poll.

Connecting all hardware devices directly to the CPU is not possible. Instead, a separate _interrupt controller_ aggregates the interrupts from all devices and then notifies the CPU:

```
                                    ____________             _____
               Timer ------------> |            |           |     |
               Keyboard ---------> | Interrupt  |---------> | CPU |
               Other Hardware ---> | Controller |           |_____|
               Etc. -------------> |____________|

```

Most interrupt controllers are programmable, which means they support different priority levels for interrupts. For example, this allows to give timer interrupts a higher priority than keyboard interrupts to ensure accurate timekeeping.

Unlike exceptions, hardware interrupts occur _asynchronously_. This means they are completely independent from the executed code and can occur at any time. Thus, we suddenly have a form of concurrency in our kernel with all the potential concurrency-related bugs. Rust’s strict ownership model helps us here because it forbids mutable global state. However, deadlocks are still possible.

---

### The 8259 PIC

The [Intel 8259](https://en.wikipedia.org/wiki/Intel_8259) is a programmable interrupt controller (PIC) introduced in 1976. It has long been replaced by the newer [APIC](https://en.wikipedia.org/wiki/Intel_APIC_Architecture), but its interface is still supported on current systems for backwards compatibility reasons. The 8259 PIC is significantly easier to set up than the APIC, so I used it to get familiar switching to the APIC.

The 8259 has eight interrupt lines and several lines for communicating with the CPU. The typical systems back then were equipped with two instances of the 8259 PIC, one primary and one secondary PIC, connected to one of the interrupt lines of the primary:

```
                     ____________                          ____________
Real Time Clock --> |            |   Timer -------------> |            |
ACPI -------------> |            |   Keyboard-----------> |            |      _____
Available --------> | Secondary  |----------------------> | Primary    |     |     |
Available --------> | Interrupt  |   Serial Port 2 -----> | Interrupt  |---> | CPU |
Mouse ------------> | Controller |   Serial Port 1 -----> | Controller |     |_____|
Co-Processor -----> |            |   Parallel Port 2/3 -> |            |
Primary ATA ------> |            |   Floppy disk -------> |            |
Secondary ATA ----> |____________|   Parallel Port 1----> |____________|

```

This graphic shows the typical assignment of interrupt lines. Most of the 15 lines have a fixed mapping, e.g., line 4 of the secondary PIC is assigned to the mouse.

Each controller can be configured through two [I/O ports](https://os.phil-opp.com/testing/#i-o-ports), one “command” port and one “data” port. For the primary controller, these ports are `0x20` (command) and `0x21` (data). For the secondary controller, they are `0xa0` (command) and `0xa1` (data).

---

### Implementation

The default configuration of the PICs is not usable because it sends interrupt vector numbers in the range of 0–15 to the CPU. These numbers are already occupied by CPU exceptions. For example, number 8 corresponds to a double fault. To fix this overlapping issue, I needed to remap the PIC interrupts to different numbers. The actual range doesn’t matter as long as it does not overlap with the exceptions, but typically the range of 32–47 is chosen, because these are the first free numbers after the 32 exception slots.

The configuration happens by writing special values to the command and data ports of the PICs. Fortunately, there is already a crate called [`pic8259`](https://docs.rs/pic8259/0.10.1/pic8259/), so I didn’t need to write the initialization sequence ourselves. 

I also check [its source code](https://docs.rs/crate/pic8259/0.10.1/source/src/lib.rs) out of curiosity. It is fairly small and well documented.

I added the crate as a dependency the project:

```toml
# in Cargo.toml

[dependencies]
pic8259 = "0.10.1"
```

The main abstraction provided by the crate is the [`ChainedPics`](https://docs.rs/pic8259/0.10.1/pic8259/struct.ChainedPics.html) struct that represents the primary/secondary PIC layout as above. It is designed to be used in the following way:

```rust
// in src/interrupts.rs

use pic8259::ChainedPics;
use spin;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
```

As noted above, I set the offsets for the PICs to the range 32–47. By wrapping the `ChainedPics` struct in a `Mutex`, I can get safe mutable access (through the [`lock` method](https://docs.rs/spin/0.5.2/spin/struct.Mutex.html#method.lock)), which I needed in the next step. The `ChainedPics::new` function is unsafe because wrong offsets could cause undefined behavior.

I then initialized the 8259 PIC in the `init` function:

```rust
// in src/lib.rs

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() }; // new
}
```

I used the [`initialize`](https://docs.rs/pic8259/0.10.1/pic8259/struct.ChainedPics.html#method.initialize) function to perform the PIC initialization. Like the `ChainedPics::new` function, this function is also unsafe because it can cause undefined behavior if the PIC is misconfigured.

---
### Enabling Interrupts

Interrupts are still disabled in the CPU configuration. This means that the CPU does not listen to the interrupt controller at all, so no interrupts can reach the CPU. So I changed that:

```rust
// in src/lib.rs

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();     // new
}
```

The `interrupts::enable` function of the `x86_64` crate executes the special `sti` instruction (“set interrupts”) to enable external interrupts. 
Then I `cargo run, I saw that a double fault occured:

![[Screenshot From 2026-05-18 10-59-33 1.png]]
The reason for this double fault is that the hardware timer (the [Intel 8253](https://en.wikipedia.org/wiki/Intel_8253), to be exact) is enabled by default, so I started receiving timer interrupts as soon as I enabled interrupts. Since I didn’t define a handler function for it yet, the double fault handler is invoked.

---

### Handling Timer Interrupts

The timer uses line 0 of the primary PIC. This means that it arrives at the CPU as interrupt 32 (0 + offset 32). Instead of hardcoding index 32, I stored it in an `InterruptIndex` enum:

```rust
// in src/interrupts.rs

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
```

The enum is a [C-like enum](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations) so that I can directly specify the index for each variant. The `repr(u8)` attribute specifies that each variant is represented as a `u8`. I will add more variants for other interrupts in the future.

Then I added a handler function for the timer interrupt:

```rust
// in src/interrupts.rs

use crate::print;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        […]
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler); // new

        idt
    };
}

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    print!(".");
}
```

The `timer_interrupt_handler` has the same signature as the exception handlers, because the CPU reacts identically to exceptions and external interrupts (the only difference is that some exceptions push an error code). The [`InterruptDescriptorTable`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.InterruptDescriptorTable.html) struct implements the [`IndexMut`](https://doc.rust-lang.org/core/ops/trait.IndexMut.html) trait, so I can access individual entries through array indexing syntax.

In timer interrupt handler, I printed a dot to the screen. As the timer interrupt happens periodically, I expected to see a dot appearing on each timer tick. However, when I ran it, I saw that only a single dot is printed.

---

### End of Interrupt

The reason is that the PIC expects an explicit “end of interrupt” (EOI) signal from the interrupt handler. This signal tells the controller that the interrupt was processed and that the system is ready to receive the next interrupt. So the PIC thinks we're still busy processing the first timer interrupt and waits patiently for the EOI signal before sending the next one.

To send the EOI, I used the static `PICS` struct again:

```rust
// in src/interrupts.rs

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    print!(".");

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}
```

The `notify_end_of_interrupt` figures out whether the primary or secondary PIC sent the interrupt and then uses the `command` and `data` ports to send an EOI signal to the respective controllers. If the secondary PIC sent the interrupt, both PICs need to be notified because the secondary PIC is connected to an input line of the primary PIC.

I needed to be careful to use the correct interrupt vector number, otherwise I could accidentally delete an important unsent interrupt or cause the system to hang. This is the reason that the function is unsafe.

Then when I executed `cargo run`, I saw dots periodically appearing on the screen:
![[Screenshot From 2026-05-18 16-11-24 1.png]]

---

### Configuring the Timer

The hardware timer that I used is called the _Programmable Interval Timer_, or PIT, for short. Like the name says, it is possible to configure the interval between two interrupts. I didn’t go into details because I will switch to the [APIC timer](https://wiki.osdev.org/APIC_timer) soon,

But the OSDev wiki has an extensive article about the [configuring the PIT](https://wiki.osdev.org/Programmable_Interval_Timer).
Maybe If I'm interested in the future I'll go deep in it.

## Deadlocks

I now have a form of concurrency in my kernel: The timer interrupts occur asynchronously, so they can interrupt the `_start` function at any time. Fortunately, Rust’s ownership system prevents many types of concurrency-related bugs at compile time. One notable exception is deadlocks. Deadlocks occur if a thread tries to acquire a lock that will never become free. Thus, the thread hangs indefinitely.

I can already provoke a deadlock in my kernel. I remembered, my `println` macro calls the `vga_buffer::_print` function, which [locks a global `WRITER`](https://os.phil-opp.com/vga-text-mode/#spinlocks) using a spinlock:

```rust
// in src/vga_buffer.rs

[…]

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
```

It locks the `WRITER`, calls `write_fmt` on it, and implicitly unlocks it at the end of the function. Now imagine that an interrupt occurs while the `WRITER` is locked and the interrupt handler tries to print something too:

|Timestep|_start|interrupt_handler|
|---|---|---|
|0|calls `println!`||
|1|`print` locks `WRITER`||
|2||**interrupt occurs**, handler begins to run|
|3||calls `println!`|
|4||`print` tries to lock `WRITER` (already locked)|
|5||`print` tries to lock `WRITER` (already locked)|
|…||…|
|_never_|_unlock `WRITER`_||

The `WRITER` is locked, so the interrupt handler waits until it becomes free. But this never happens, because the `_start` function only continues to run after the interrupt handler returns. Thus, the entire system hangs.

### Provoking a Deadlock

I easily provoked such a deadlock in the kernel by printing something in the loop at the end of the `_start` function:

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    […]
    loop {
        use rusty_os::print;
        print!("-");        // new
    }
}
```

When I ran it in QEMU, I got an output of the form:

![[qemu-deadlock.png]]

I saw that only a limited number of hyphens are printed until the first timer interrupt had occured. Then the system hangs because the timer interrupt handler deadlocks when it tries to print a dot. This is the reason that I saw no dots in the above output.

---

### Fixing the Deadlock

To avoid this deadlock, I  disabled interrupts as long as the `Mutex` is locked:

```rust
// in src/vga_buffer.rs

/// Prints the given formatted string to the VGA text buffer
/// through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;   // new

    interrupts::without_interrupts(|| {     // new
        WRITER.lock().write_fmt(args).unwrap();
    });
}
```

The [`without_interrupts`](https://docs.rs/x86_64/0.14.2/x86_64/instructions/interrupts/fn.without_interrupts.html) function takes a [closure](https://doc.rust-lang.org/book/ch13-01-closures.html) and executes it in an interrupt-free environment. I used it to ensure that no interrupt can occur as long as the `Mutex` is locked.

I applied the same change to the serial printing function to ensure that no deadlocks occur with it either:

```rust
// in src/serial.rs

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;       // new

    interrupts::without_interrupts(|| {         // new
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}
```

Disabling interrupts shouldn’t be a general solution. The problem is that it increases the worst-case interrupt latency, i.e., the time until the system reacts to an interrupt. Therefore, interrupts should only be disabled for a very short time.

---

### The `hlt` instruction

Until now, I used a simple empty loop statement at the end of the `_start` and `panic` functions. This causes the CPU to spin endlessly, and thus works as expected. But it is also very inefficient, because the CPU continues to run at full speed even though there’s no work to do.

What I really wanted to do is to halt the CPU until the next interrupt arrives. This allows the CPU to enter a sleep state in which it consumes much less energy. The [`hlt` instruction](https://en.wikipedia.org/wiki/HLT_\(x86_instruction\)) does exactly that. So I  used this instruction to create an energy-efficient endless loop:

```rust
// in src/lib.rs

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
```

The `instructions::hlt` function is just a [thin wrapper](https://github.com/rust-osdev/x86_64/blob/5e8e218381c5205f5777cb50da3ecac5d7e3b1ab/src/instructions/mod.rs#L16-L22) around the assembly instruction. It is safe because there’s no way it can compromise memory safety.

Then I used this `hlt_loop` instead of the endless loops in the `_start` and `panic` functions:

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    […]

    println!("It did not crash!");
    rusty_os::hlt_loop();            // new
}


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rusty_os::hlt_loop();            // new
}

```

I updated the `lib.rs` as well:

```rust
// in src/lib.rs

/// Entry point for `cargo test`
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    hlt_loop();         // new
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();         // new
}
```

---

### Keyboard Input

Now that I am able to handle interrupts from external devices, I'm finally able to add support for keyboard input. This will allow me to interact with my kernel for the first time.

> I only learned how to handle [PS/2](https://en.wikipedia.org/wiki/PS/2_port) keyboards here, not USB keyboards. However, the mainboard emulates USB keyboards as PS/2 devices to support older software, so I can safely ignore USB keyboards until I have USB support in our kernel.

Like the hardware timer, the keyboard controller is already enabled by default. So when I press a key, the keyboard controller sends an interrupt to the PIC, which forwards it to the CPU. The CPU looks for a handler function in the IDT, but the corresponding entry is empty. Therefore, a double fault occurs.

So I added a handler function for the keyboard interrupt. It was quite similar to how we defined the handler for the timer interrupt; it just uses a different interrupt number:

```rust
// in src/interrupts.rs

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard, // new
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        […]
        // new
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    print!("k");

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
```

The keyboard uses line 1 of the primary PIC. This means that it arrives at the CPU as interrupt 33 (1 + offset 32). I added this index as a new `Keyboard` variant to the `InterruptIndex` enum. I didn’t need to specify the value explicitly, since it defaults to the previous value plus one, which is also 33. In the interrupt handler, I printed a `k` and send the end of interrupt signal to the interrupt controller.

Then I saw that a `k` appears on the screen when I pressed a key. However, this only worked for the first key I pressed. Even if I continued to press keys, no more `k`s appear on the screen. This was because the keyboard controller won’t send another interrupt until I have read the so-called _scancode_ of the pressed key.

### Reading the Scancodes

To find out _which_ key was pressed, I needed to query the keyboard controller. I ded this by reading from the data port of the PS/2 controller, which is the [I/O port](https://os.phil-opp.com/testing/#i-o-ports) with the number `0x60`:

```rust
// in src/interrupts.rs

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    print!("{}", scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
```

I used the [`Port`](https://docs.rs/x86_64/0.14.2/x86_64/instructions/port/struct.Port.html) type of the `x86_64` crate to read a byte from the keyboard’s data port. This byte is called the [_scancode_](https://en.wikipedia.org/wiki/Scancode) and it represents the key press/release. I didn’t do anything with the scancode yet, other than print it to the screen:
![[Screenshot From 2026-05-18 16-48-13 1.png]]

I saw that adjacent keys have adjacent scancodes and that pressing a key causes a different scancode than releasing it. But how do I translate the scancodes to the actual key actions exactly?

### Interpreting the Scancodes

There are three different standards for the mapping between scancodes and keys, the so-called _scancode sets_. All three go back to the keyboards of early IBM computers: the [IBM XT](https://en.wikipedia.org/wiki/IBM_Personal_Computer_XT), the [IBM 3270 PC](https://en.wikipedia.org/wiki/IBM_3270_PC), and the [IBM AT](https://en.wikipedia.org/wiki/IBM_Personal_Computer/AT). Later computers fortunately did not continue the trend of defining new scancode sets, but rather emulated the existing sets and extended them. Today, most keyboards can be configured to emulate any of the three sets.

By default, PS/2 keyboards emulate scancode set 1 (“XT”). In this set, the lower 7 bits of a scancode byte define the key, and the most significant bit defines whether it’s a press (“0”) or a release (“1”). Keys that were not present on the original [IBM XT](https://en.wikipedia.org/wiki/IBM_Personal_Computer_XT) keyboard, such as the enter key on the keypad, generate two scancodes in succession: a `0xe0` escape byte and then a byte representing the key. For a list of all set 1 scancodes and their corresponding keys, check out the [OSDev Wiki](https://wiki.osdev.org/Keyboard#Scan_Code_Set_1).

To translate the scancodes to keys, I used a `match` statement:

```rust
// in src/interrupts.rs

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // new
    let key = match scancode {
        0x02 => Some('1'),
        0x03 => Some('2'),
        0x04 => Some('3'),
        0x05 => Some('4'),
        0x06 => Some('5'),
        0x07 => Some('6'),
        0x08 => Some('7'),
        0x09 => Some('8'),
        0x0a => Some('9'),
        0x0b => Some('0'),
        _ => None,
    };
    if let Some(key) = key {
        print!("{}", key);
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
```

The above code translates keypresses of the number keys 0-9 and ignores all other keys. It uses a [match](https://doc.rust-lang.org/book/ch06-02-match.html) statement to assign a character or `None` to each scancode. It then uses [`if let`](https://doc.rust-lang.org/book/ch19-01-all-the-places-for-patterns.html#conditional-if-let-expressions) to destructure the optional `key`. By using the same variable name `key` in the pattern, we [shadow](https://doc.rust-lang.org/book/ch03-01-variables-and-mutability.html#shadowing) the previous declaration, which is a common pattern for destructuring `Option` types in Rust.

Then I tested it and it printed the numbers I typed:

![[Screenshot From 2026-05-18 16-47-38 2.png]]
Translating the other keys works in the same way. Fortunately, there is a crate named [`pc-keyboard`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/) for translating scancodes of scancode sets 1 and 2, so I don’t have to implement this ourselves. To use the crate, I added it to `Cargo.toml` and import it in the `lib.rs`:

```toml
# in Cargo.toml

[dependencies]
pc-keyboard = "0.7.0"
```

Then I used this crate to rewrite the keyboard_interrupt_handler`:

```rust
// in/src/interrupts.rs

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(ScancodeSet1::new(),
                layouts::Us104Key, HandleControl::Ignore)
            );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
```

I used the `lazy_static` macro to create a static [`Keyboard`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/struct.Keyboard.html) object protected by a Mutex. I initialized the `Keyboard` with a US keyboard layout and the scancode set 1. The [`HandleControl`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/enum.HandleControl.html) parameter allows to map `ctrl+[a-z]` to the Unicode characters `U+0001` through `U+001A`. I didn’t want to do that, so I used the `Ignore` option to handle the `ctrl` like normal keys.

On each interrupt, I lock the Mutex, read the scancode from the keyboard controller, and pass it to the [`add_byte`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/struct.Keyboard.html#method.add_byte) method, which translates the scancode into an `Option<KeyEvent>`. The [`KeyEvent`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/struct.KeyEvent.html) contains the key which caused the event and whether it was a press or release event.

To interpret this key event, I pass it to the [`process_keyevent`](https://docs.rs/pc-keyboard/0.7.0/pc_keyboard/struct.Keyboard.html#method.process_keyevent) method, which translates the key event to a character, if possible. For example, it translates a press event of the `A` key to either a lowercase `a` character or an uppercase `A` character, depending on whether the shift key was pressed.

With this modified interrupt handler, I can now write text:

![[Screenshot From 2026-05-18 16-56-15.png]]

---

