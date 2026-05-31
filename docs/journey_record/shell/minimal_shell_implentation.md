
I tried to create a minimal shell.

`GOAL`: 
- print a prompt like `>`.
- on Enter key, process the command and clear the buffer.
- support basic editing (backspace).

---

### Implementation

First I create a separate module for shell - `shell.rs`. 

```rust
pub mod shell;
```


Then I created `KeyboardBuffer` struct which stores the key pressed.

```rust
pub struct KeyboardBuffer {
    buffer: [char; 256], // Simple circular buffer for now
    head: usize,
    tail: usize,
}
```

I implemented `new`, `push_back`, `pop` method for `KeyboardBuffer`: 

```rust
impl KeyboardBuffer {
    pub const fn new() -> Self {
        KeyboardBuffer {
            buffer: ['\0'; 256],
            head: 0,
            tail: 0,
        }
    }

    pub fn push_back(&mut self, c: char) {
        self.buffer[self.head] = c;
        self.head = (self.head + 1) % 256;
    }

    pub fn pop(&mut self) -> Option<char> {
        if self.head == self.tail {
            return None;
        }
        let c = self.buffer[self.tail];
        self.tail = (self.tail + 1) % 256;
        Some(c)
    }
}

```

`push_back`: Stores the key pressed in the buffer mod the size.
`pop`: Removes and returns the character from front.

---

Then I created global static input queue for the shell.

```rust
lazy_static! {
    pub static ref KEYBOARD_INPUT: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer::new());
}

```


I created struct `Shell`:

```rust

pub struct Shell {
    input_line: [u8; 256],
    len: usize,
}

```

I used array of size 256 because using `String` would cause some error or bug when allocating memory on heap which I don't know why.

Then I implemented the following method for `Shell`:

```rust
impl Shell {
    pub fn new() -> Self {
        Shell {
            input_line: [0u8; 256],
            len: 0,
        }
    }

    pub fn handle_input(&mut self) {
        let mut input = KEYBOARD_INPUT.lock();
        while let Some(c) = input.pop() {
            match c {
                '\n' => {
                    self.execute_command();
                    self.input_line = [0u8; 256];
                    self.len = 0;
                    self.print_prompt();
                }
                '\x08' => {
                    // Backspace

                    if self.len > 0 {
                        // scan backwards to find the start of the last UTF-8 character
                        let mut i = self.len - 1;
                        while i > 0 && (self.input_line[i] & 0b1100_0000) == 0b1000_0000 {
                            i -= 1;
                        }
                        self.len = i;
                    }
                }

                c => {
                    print!("{}", c);
                    self.len += c.encode_utf8(&mut self.input_line[self.len..]).len();
                }
            }
        }
    }

    fn execute_command(&self) {
        use core::str;
        let cmd: &str = unsafe { str::from_utf8_unchecked(&self.input_line[..self.len]).trim() };

        match cmd {
            "help" => println!("\nCommands: help, echo, mem, clear, panic"),
            "clear" => crate::vga_buffer::clear_screen(),
            "panic" => panic!("Manaul panic triggered!"),
            _ if cmd.starts_with("echo ") => {
                println!("\n{}", &cmd[5..]);
            }

            _ => println!("\nUnknown command: {}", cmd),
        }
    }

    fn print_prompt(&self) {
        print!("\n> ");
    }
}

```


# Shell Input System

## Overview

This shell implementation provides a simple command-line interface for the kernel without using heap allocations. Instead of storing user input in a dynamically allocated `String`, it uses a fixed-size byte buffer.

This design is common in operating systems and embedded systems because it:

- Avoids heap allocations during input processing
- Prevents allocation failures while handling keyboard input
- Provides predictable memory usage
- Reduces allocator overhead
- Makes debugging easier

---

# Shell Structure

```rust
pub struct Shell {
    input_line: [u8; 256],
    len: usize,
}
```

## Fields

### `input_line`

```rust
input_line: [u8; 256]
```

A fixed-size byte buffer that stores the current command being typed.

Example:

User types:

```text
echo hello
```

Buffer contents:

```text
[e][c][h][o][ ][h][e][l][l][o]
```

Remaining bytes remain unused.

Maximum command length:

```text
256 bytes
```

---

### `len`

```rust
len: usize
```

Tracks the number of bytes currently used in the buffer.

Example:

```text
Command: help
```

Buffer:

```text
[h][e][l][p]
```

Length:

```rust
len = 4
```

This prevents reading uninitialized portions of the buffer.

---

# Constructor

```rust
pub fn new() -> Self {
    Shell {
        input_line: [0u8; 256],
        len: 0,
    }
}
```

## Purpose

Creates an empty shell.

Initial state:

```text
Buffer = all zeros
Length = 0
```

No heap memory is allocated.

---

# Input Handling

```rust
pub fn handle_input(&mut self)
```

Processes all available keyboard input from the keyboard buffer.

---

## Reading Characters

```rust
let mut input = KEYBOARD_INPUT.lock();

while let Some(c) = input.pop() {
```

### Explanation

The keyboard interrupt handler places characters into a global keyboard queue.

Example:

```text
Keyboard Interrupt
       ↓
KEYBOARD_INPUT
       ↓
Shell reads characters
```

The shell continuously removes characters until the queue is empty.

---

# Enter Key Processing

```rust
'\n' => {
    self.execute_command();
    self.input_line = [0u8; 256];
    self.len = 0;
    self.print_prompt();
}
```

Triggered when the user presses Enter.

Example:

```text
> help
```

Steps:

1. Execute command
2. Clear buffer
3. Reset length
4. Print new prompt

Result:

```text
> help
Commands: help, echo, mem, clear, panic

>
```

---

# Backspace Processing

```rust
'\x08' => {
```

Represents the Backspace key.

---

## Why UTF-8 Handling Is Needed

Characters are not always one byte.

Examples:

| Character | Bytes |
|------------|---------|
| A | 1 |
| B | 1 |
| é | 2 |
| 世 | 3 |
| 🚀 | 4 |

Removing a single byte may corrupt a character.

---

## Backspace Logic

```rust
if self.len > 0 {
```

Ensure buffer is not empty.

---

### Start At Last Byte

```rust
let mut i = self.len - 1;
```

Move to the last byte currently stored.

---

### Skip UTF-8 Continuation Bytes

```rust
while i > 0 &&
      (self.input_line[i] & 0b1100_0000)
      == 0b1000_0000
{
    i -= 1;
}
```

UTF-8 continuation bytes always begin with:

```text
10xxxxxx
```

Binary:

```text
0b1000_0000
```

This loop walks backwards until the first byte of the character is found.

---

### Update Length

```rust
self.len = i;
```

Effectively removes the last UTF-8 character.

Example:

Before:

```text
hello🚀
```

After Backspace:

```text
hello
```

---

# Storing Typed Characters

```rust
c => {
    print!("{}", c);

    self.len += c.encode_utf8(
        &mut self.input_line[self.len..]
    ).len();
}
```

---

## Echo Character

```rust
print!("{}", c);
```

Displays the typed character on screen.

Example:

User presses:

```text
A
```

Screen:

```text
A
```

---

## Encode UTF-8

```rust
c.encode_utf8(...)
```

Converts a Rust `char` into UTF-8 bytes and stores them directly inside the buffer.

Examples:

### ASCII

```rust
'A'
```

Stored:

```text
41
```

Length:

```text
1 byte
```

---

### Unicode

```rust
'🚀'
```

Stored:

```text
F0 9F 9A 80
```

Length:

```text
4 bytes
```

---

## Update Length

```rust
self.len += ...
```

Keeps track of how many bytes are currently occupied.

---

# Command Execution

```rust
fn execute_command(&self)
```

Converts stored bytes into a string and executes commands.

---

## Convert Buffer To String

```rust
let cmd: &str =
    unsafe {
        str::from_utf8_unchecked(
            &self.input_line[..self.len]
        )
    }
    .trim();
```

### Why Safe Here?

All bytes were written using:

```rust
encode_utf8()
```

which always produces valid UTF-8.

Therefore:

```rust
from_utf8_unchecked()
```

is safe and avoids validation overhead.

---

## Trim Whitespace

```rust
.trim()
```

Removes:

```text
spaces
tabs
newlines
```

from both ends.

Example:

```text
"  help  "
```

becomes:

```text
"help"
```

---

# Supported Commands

## Help

```rust
"help"
```

Output:

```text
Commands: help, echo, mem, clear, panic
```

---

## Clear

```rust
"clear"
```

Calls:

```rust
crate::vga_buffer::clear_screen()
```

Clears the display.

---

## Panic

```rust
"panic"
```

Triggers:

```rust
panic!("Manual panic triggered!")
```

Useful for testing:

- panic handlers
- stack traces
- exception handling

---

## Echo

```rust
echo something
```

Example:

```text
> echo hello
```

Output:

```text
hello
```

Implementation:

```rust
cmd.starts_with("echo ")
```

Then:

```rust
&cmd[5..]
```

extracts everything after:

```text
echo
```

---

## Unknown Command

Fallback case:

```rust
Unknown command: xyz
```

Example:

```text
> abc
Unknown command: abc
```

---

# Prompt Display

```rust
fn print_prompt(&self) {
    print!("\n> ");
}
```

Displays:

```text
>
```

to indicate readiness for the next command.

---

# Memory Characteristics

## Previous Implementation

```rust
String
```

Characteristics:

- Heap allocation required
- Dynamic resizing
- Possible allocation failures
- Allocator dependency

---

## Current Implementation

```rust
[u8; 256]
```

Characteristics:

- No heap allocation
- Constant memory usage
- Predictable behavior
- Safer in kernel environments

Memory consumption:

```text
256 bytes buffer
8 bytes length (64-bit)
≈264 bytes total
```

---

# Input Flow

```text
Keyboard Interrupt
        │
        ▼
KEYBOARD_INPUT Queue
        │
        ▼
Shell::handle_input()
        │
        ▼
Store UTF-8 bytes
        │
        ▼
Press Enter
        │
        ▼
execute_command()
        │
        ▼
Run Command
        │
        ▼
Clear Buffer
        │
        ▼
Display Prompt
```

---

# Advantages Of This Design

1. No heap allocations
2. Fixed memory usage
3. UTF-8 compatible
4. Safe Backspace handling
5. Suitable for kernel development
6. Low overhead
7. Deterministic behavior
8. Easy to debug
9. Avoids allocator-related failures
10. Works well in interrupt-driven systems

This architecture is much closer to how real operating system kernels implement console and shell input handling.

---
