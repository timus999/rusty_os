(May 26, 2026)

---

This is the Day 20 of my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

Since `vga text buffer` and `interrupt handler` was implemented, I thought I will try to build a shell for interacting with the kernel.

For that, I added a new function called `delete_char` which will delete a char when pressing backspace key:

```rust
// in src/vga_buffer.rs
// impl Writer

pub fn delete_char(&mut self) {
        if self.column_position > 1 {
            self.column_position -= 1;
            let row = BUFFER_HEIGHT - 1;
            let col = self.column_position;

            let blank = ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            };

            self.char_ptr(row, col).write(blank);
        }
    }
```

First it checks if the column position is greater than 1 or not because at position 1 I have `>` prompt, so I don't want to delete char further that.
We subtract the column position by 1 because we want to cursor to be 1 position back. Then we set the row to be `BUFFER_HEIGHT - 1` because we only want to delete on the last row. I will implement deletion for multiple rows in the future.
Then I created `ScreenChar` struct for blank character and wrote in the deletion position.

Then I created separate module for keyboard interrupt:

```rust
// in interrupt.rs
pub mod keyboard;
```

Then added extra match arm for backspace key:

```rust
// in interrupt/keyboard.rs
// iniside keyboard_interrupt_handler fn

	DecodedKey::Unicode(character) => match character {
		'\x08' => crate::vga_buffer::WRITER.lock().delete_char(), // new
		_ => print!("{}", character),
	},


```

When I ran it, It successfully removed a character.

---

This much was for toady. I had some other work to do. So I could not give full time today.
