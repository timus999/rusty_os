
Before moving forward on OS development, I need to fully understand what `VGA` is.

## The Display

The **Video Graphics Array(VGA)** is an analog computer display standard marketed in 1987 by IBM. It is called "Array" because it was originally developed as a single chip, replacing dozens of logic chips in a Industry Standard Architecture (ISA) board that the `MDA, CGA, EGA` used. Because this was all on a single ISA board, it was very easy to connect it to the motherboard.

The VGA consists of the **video buffer**, **video DAC**, **CRT Controller**, **Sequencer unit**, **Graphics Controller**, and an **Attribute Controller**.

### Video buffer

The video buffer is a segment of memory mapped as Video Memory. We can change what region of memory is mapped to video memory.

**_AT startup, the BIOS maps it to 0xA0000, which means that video memory is mapped to 0xA0000._**

Text Mode 7 uses two bytes per character.
The first byte is attribute bytes which splits into two segments, the foreground color and background color.
The second byte is an 8-bit ASCII value of the character to print.

n VGA mode 3, the linear text buffer is located in physical at 0xB8000. Reading and writing to and from this address will provide direct manipulation of on screen text. To access a particular character on the screen from X and Y coordinates is simple using the following formula:

`position = (y_position * characters_per_line) + x_position;`

### Video DAC

The Video Digital to Analog Converter (DAC) contains the **color palette** that is used to convert the vidoe data into an analog video signal that is sent to the display.
This signal indicated the **red**, **green** and **blue** intensities in analog form.

### CRT Controller

This controller generates horizontal and vertical synchronization signal timings, addressing for the video buffer, cursor and underline timings.

### Sequencer

The Sequencer generates basic memory timings for video memory and the character clock for controlling regenerative buffer fetches. It allows the system to access memory during active display intervals.

### Graphics Controller

This is the interface between video memory and the attribute controller, and between video memory and the CPU. 
During active display times, memory data is sent form the video buffer (Video Memory) and sent to the Attribute Controller.

---
## Video Modes

A `Video Mode` is a specification of display. That is, it describes how `Video Memory` is referenced, and how this data is displayed by the video adapter.
The VGA supports two types of modes:
1. **APA Graphics**
2. **Text**

### APA Graphics

**All Points Addressable (APA)** is a display mode, that, on a video monitor, dot matrix, or any device that consists of a pixel array, where every cell can be referenced individually. In the case of video display, where very cell represents a `pixel`, where every pixel can be manipulated directly. 
Because of this, almost all graphic modes use this method. By modifying this pixel buffer, we effectively modify individual pixels on screen.

### Text Mode

A Text Mode is a display mode where the content on the screen is internally represented in terms of characters rather then pixels, as with APA.

A Video Controller implementing text mode use two buffers:
- A character map representing the pixels for each individual character to be displayed
- A buffer that represents what characters are in each cell.

By changing the character map buffer, we effectively change the characters themselves, allowing us to create a new character set. By changing the Screen Buffer, which represents what characters are in each cell, we effectively change what characters are displayed on screen. 
Some text modes also allow attributes, which may provide a character color, or even blinking underlined, inversed, brightened, etc..


---
## MDA, CGA, EGA

### MDA - Theory

Back in 1981, IBM developed a standard video display card for the PC. They were the **Monochrome Display Adapter (MDA), and Monochrome Display and Printer Adapter (MDPA).**

The `MDA` did not have any graphics mode of any kind. It only had a single text mode, (Mode 7) which could display 80 columns by 25 lines of high resolution text characters.

### CGA - Theory

In 1981, IBM also developed the **Color Graphics Adapter (CGA)**, considered the first color display standard for PC's.
The CGA only supported a Color Palette of 16 colors, because it was limited to 4 bytes per pixel.

CGA supported two text modes and two graphics modes, including:

- 40x25 characters (16 color) text mode
- 18x25 characters (16 color) text mode
- 320x200 pixels (4 colors) graphics modes
- 640x200 pixels (Monochrome) graphics mode

### EGA - Theory

Introduced in 1984 by IBM, The **Enhanced Graphics Adapter (EGA)**. produced a display of 16 colors at a resolution up to 640x350 pixels.

**Note: The VGA adapters are backward compatible, similar to the 80x86 microprocessor family. Because of this, and to ensure backward compatibility, the BIOS starts up in Mode 7 (Originally form the MDA), which supports 80 columns, by 25 lines. This is important to us, because this is the mode we are in!**

---
## VGA Memory Addressing

Video Memory used by the VGA Controller is mapped to the PC's memory from 0xA0000 to 0xBFFFF.

Typically, the Video Memory is mapped as the following:

- **0xA0000 - 0xBFFFF** Video Memory used for graphics modes
- **0xB0000 - 0xB7777** Monochrome Text mode
- **0xB8000 - 0xBFFFF** Color text mode and CGA compatible graphics modes


---
## Colors

Each character has a color byte. This color byte is split up in foreground color and background color.  

The layout of the byte, using the standard colour palette:

Bit 76543210
    ||||||||
    |||||^^^-fore color
    ||||^----fore color bright bit
    |^^^-----back color
    ^--------back color bright bit OR enables blinking Text

Its easy to write to BL, the Color Nibbles(4Bit), in a Hex Value.

For Example:  

```
0x01 sets the background to black and the fore color to blue  
0x10 sets the background to blue and the fore color to black   
0x11 sets both to blue.
```


The default display colors set by the BIOS upon booting are 0x0F: 0 (black) for the background and 7 (White) + 8 (Bright) for the foreground.  
  

In text mode 0, the following standard color palette is available for use. You can change this palette with VGA commands.

| Number | Name       | Number + bright bit | Name          |
| ------ | ---------- | ------------------- | ------------- |
| 0      | Black      | 0+8=8               | Dark Gray     |
| 1      | Blue       | 1+8=9               | Light Blue    |
| 2      | Green      | 2+8=A               | Light Green   |
| 3      | Cyan       | 3+8=B               | Light Cyan    |
| 4      | Red        | 4+8=C               | Light Red     |
| 5      | Magenta    | 5+8=D               | Light Magenta |
| 6      | Brown      | 6+8=E               | Yellow        |
| 7      | Light Gray | 7+8=F               | White         |
