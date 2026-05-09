### Real mode, Protected mode, Long mode

- 8086/8088 (1978) -> Real mode ( 16-bit )
- 80286 (1982) -> Protected mode ( 16-bit )
- 80368 (1985) -> 32-bit Protected mode 
- AMD64/Intel64 (2003) -> Long Mode ( 64-bit )

### Real mode

- Initial operating mode of x86 CPUs
- called "real" because address are "real" physical addresses
- no translation, no protection

#### Limitations

- No memory protection: Any code can write anywhere
- No multitasking: No hardware support for multitasking
- 16-bit addressing: Maximum 1MB RAM
- No virtual memory: All addresses are physical
- Limited addressing mode: Only (BX+SI), (BP+DI) etc.

#### Why does Real Mode still exists?

Compatibility! Every x86 CPU starts in real mode to boot.

Power On -> CPU in Real mode -> BIOS runs -> Bootloader -> Switch to Protected/Long mode.

This allows modern CPUs to run legacy boot loaders, DOS program and BIOS interrupts

### Protected Mode ( The Revolution )

(introduced with 80286, enhanced in 80386 )

- memory protections
- Virtual memory (paging in 386+)
- Multitasking support
- 32-bit addressing (4GB address space)
- Privilege levels (rings 0-3)

#### Protection Features

- Segment Limit Checking: Can't access beyond segment boundary
- Type Checking: Code can't be written, data can't be executed
- Privilege levels: Ring 0 - Kernel ( can do anything ) Ring 3 - User applications (restricted)
- Page Protection: Read-only pages, user/supervisor pages

#### Protection Mode Limitations (original 20286)
- still 16-bit
- No paging
- Difficult to switch back to real mode.

### Long Mode

extends x86 to 64-bit while maintaining compatibility with 32-bit and 16-bit modes.

It has 2 sub-modes:
- 64-bit mode ( pure 64-bit OS, 64-bit apps )
- Compatibility mode ( 64-bit OS running 32/16-bit apps )

#### Key Features
- 64-bit virtual addresses
- Terabytes of physical RAM
- 16 general purpose register
- Register extensions
- No segmentation
- Page tables with 4 levels

