use core::fmt;
use core::ptr::{addr_of_mut, NonNull};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::VolatilePtr;

#[allow(dead_code)] // disable warning for unused vairant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)] // this sets the underlying type to an 8-bit integer (u8)
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

// This structs contains full color byte, foreground and background color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)] // data layout stay exact same as a u8
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // Since the field ordering in default structs is undefined in Rust
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)] // ensures it also has same memory layout
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT], // remove old Volatile
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: VolatilePtr<'static, Buffer>, // Wrap with VolatilePtr
}

// VolatilePtr is Send/Sync because it only holds a NonNull pointer
// VGA buffer access is safe from multiple threads as long as we use the Mutex
unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Writer {
    fn char_ptr(&mut self, row: usize, col: usize) -> VolatilePtr<'_, ScreenChar> {
        unsafe {
            let buffer_ptr = self.buffer.as_raw_ptr().as_ptr();
            let row_ptr = addr_of_mut!((*buffer_ptr).chars[row]);
            let char_ptr = addr_of_mut!((*row_ptr)[col]);
            VolatilePtr::new(NonNull::new_unchecked(char_ptr))
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.char_ptr(row, col).write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.char_ptr(row, col).read();
                self.char_ptr(row - 1, col).write(character);
            }
        }

        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.char_ptr(row, col).write(blank);
        }
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Green, Color::Black),
        buffer: unsafe { VolatilePtr::new(NonNull::new_unchecked(0xB8000 as *mut Buffer)) }, // use VolatilePtr::new()
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
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
