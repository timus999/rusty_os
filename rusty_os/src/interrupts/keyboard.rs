use crate::interrupts::{InterruptIndex, PICS};
use alloc::collections::VecDeque;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;
lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            ScancodeSet1::new(),
            layouts::Us104Key,
            HandleControl::Ignore
        ));
}

// This will be our global input queue for the shell
lazy_static! {
    pub static ref KEYBOARD_INPUT: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer::new());
}

pub struct KeyboardBuffer {
    buffer: [char; 256], // Simple circular buffer for now
    head: usize,
    tail: usize,
}

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

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            let mut input = KEYBOARD_INPUT.lock();
            match key {
                DecodedKey::RawKey(_) => {
                    // handle arrow keys, function keys, etc.. later
                }

                DecodedKey::Unicode(character) => match character {
                    '\x08' => {
                        // Backspace
                        // Notify shell about backspace

                        input.push_back('\x08');

                        crate::vga_buffer::backspace();
                    }
                    '\n' | '\r' => {
                        input.push_back('\n');
                    }
                    _ => {
                        input.push_back(character);
                    }
                },
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
