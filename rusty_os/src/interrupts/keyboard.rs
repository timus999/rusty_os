use super::*;

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use crate::vga_buffer::WRITER;
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // let key = match scancode {
    //     0x02 => Some('1'),
    //     0x03 => Some('2'),
    //     0x04 => Some('3'),
    //     0x05 => Some('4'),
    //     0x06 => Some('5'),
    //     0x07 => Some('6'),
    //     0x08 => Some('7'),
    //     0x09 => Some('8'),
    //     0x0a => Some('9'),
    //     0x0b => Some('0'),
    //     _ => None,
    // };

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::RawKey(key) => print!("{:?}", key),

                DecodedKey::Unicode(character) => match character {
                    '\x08' => crate::vga_buffer::WRITER.lock().delete_char(),
                    _ => print!("{}", character),
                },
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
