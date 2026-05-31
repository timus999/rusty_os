use crate::{print, println};
use lazy_static::lazy_static;
use pc_keyboard::{DecodedKey, KeyCode, KeyEvent};
use spin::Mutex;


use crate::interrupts::keyboard::KEYBOARD_INPUT;

lazy_static! {
    static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());
}

pub struct Shell {
    input_line: [u8; 256],
    len: usize,
}

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

pub fn tick() {
    SHELL.lock().handle_input();
}
