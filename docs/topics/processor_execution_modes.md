[Link to pdf](https://gitlab.com/opensecuritytraining/arch2001_x86-64_os_internals_slides_and_subtitles/-/blob/master/02_ProcessorModes/_Arch2001_02_ProcessorModes_01_ProcModes.pdf?ref_type=heads)

## x86 Processor Modes - Complete Theoretical Explanation

The x86 architecture has evolved over 40+ years, accumulating multiple processor modes to balance backward compatibility, security, performance, and new features. Each mode serves a specific purpose and has distinct characteristics.

---

## 1. Real Mode (8086/8088 era)

### Historical Context (1978)

When Intel created the 8086 processor, they designed a simple 16-bit architecture. Memory was scarce and expensive, so they needed a way to address more than 64KB without increasing address bus width dramatically.

### Motivation

The 8086 could only address 1MB of memory (20 address lines), but its registers were only 16 bits. Intel needed a scheme to generate 20-bit addresses from 16-bit registers, leading to the **segmented memory model**.

### How It Works

- **16-bit registers** (AX, BX, CX, DX, SI, DI, BP, SP)
    
- **20-bit address bus** → 1MB addressable memory (0x00000 to 0xFFFFF)
    
- **Segmented addressing**: `Physical Address = (Segment << 4) + Offset`
    
- **Real mode** means no protection - code can access any memory address
    
- **Only one privilege level** (everything runs as ring 0 equivalent)
    
- **Interrupts use the IVT** (Interrupt Vector Table) at address 0x0000
    

### Purpose

- **Boot process**: All x86 CPUs start in real mode after reset
    
- **BIOS compatibility**: Legacy BIOS interrupts (INT 0x10, INT 0x13, etc.)
    
- **DOS compatibility**: Old operating systems ran entirely in real mode
    
- **Transition state**: Firmware uses it before switching to protected/long mode
    

### Limitations That Drove Evolution

- **1MB memory limit** (became severe as memory prices dropped)
    
- **No memory protection** (any code can corrupt any memory)
    
- **No multitasking support** (no privilege levels)
    
- **No virtual memory** (direct physical addressing only)
    
- **16-bit registers** limited arithmetic and data processing
    

---

## 2. Protected Mode (80286/80386 era)

### Historical Context (1982-1985)

As software grew more complex, the 1MB limit of real mode became crippling. Multitasking operating systems needed memory protection to prevent one crash from bringing down the whole system.

### Motivation

- **Larger address space**: Support megabytes/gigabytes of RAM
    
- **Memory protection**: Isolate processes from each other and the kernel
    
- **Multitasking hardware support**: Built-in task switching
    
- **Virtual memory**: Allow programs to use more memory than physically available
    

### Two Generations

#### 286 Protected Mode (1982 - Limited)

- **24-bit address bus** → 16MB addressable memory
    
- **Segmented protection** with descriptor tables (GDT/LDT)
    
- **Privilege levels 0-3** (rings)
    
- **Could not switch back to real mode** without resetting the CPU
    
- **Still 16-bit registers** (awkward)
    

#### 386 Protected Mode (1985 - Full)

- **32-bit address bus** → 4GB addressable memory
    
- **32-bit registers** (EAX, EBX, etc.) and instructions
    
- **Paging support** (virtual memory)
    
- **Flat memory model option** (segments start at 0, span 4GB)
    
- **Can switch back to real mode** via special sequence
    

### Key Concepts

**Descriptor Tables**:

- **GDT (Global Descriptor Table)**: System-wide segments (code, data, TSS)
    
- **LDT (Local Descriptor Table)**: Process-specific segments
    
- **IDT (Interrupt Descriptor Table)**: Interrupt handlers with protection
    

**Privilege Levels (Rings)**:

- **Ring 0**: Kernel/OS (most privileged)
    
- **Ring 1 & 2**: Device drivers (rarely used)
    
- **Ring 3**: User applications (least privileged)
    

**Paging** (386+):

- Translates virtual addresses to physical addresses
    
- Allows swapping to disk
    
- Enables per-page protection
    
- Standard 4KB pages (also 2MB/4MB large pages)
    

### Purpose

- **Modern OS foundation**: Windows NT/2000/XP, Linux, BSD (before x86_64)
    
- **Multitasking**: Hardware task switching (though software switching is faster)
    
- **Process isolation**: One application can't crash the system
    
- **Virtual memory**: Each process sees its own 4GB address space
    

### Why It Was Revolutionary

Protected mode transformed the PC from a hobbyist machine into a serious computing platform. It enabled:

- **Preemptive multitasking** (OS controls CPU time)
    
- **Memory protection** (crashes stay contained)
    
- **Virtual memory** (run programs larger than physical RAM)
    
- **Security boundaries** (users can't access kernel data)
    

---

## 3. Virtual 8086 Mode (386+)

### Historical Context (1985)

When 386 introduced protected mode, users still needed to run DOS applications. The industry couldn't abandon millions of existing programs.

### Motivation

- **Backward compatibility**: Run real-mode DOS programs under protected-mode OS
    
- **Emulation efficiency**: Hardware-assisted legacy support
    
- **Multiple DOS sessions**: Run several DOS apps simultaneously
    

### How It Works

- **A special mode within protected mode** (not a separate CPU mode)
    
- Creates a **virtual 8086 CPU** with:
    
    - 1MB address space (0-0xFFFFF)
        
    - Segmented addressing (segment << 4 + offset)
        
    - Real-mode style interrupts (IVT)
        
    - 16-bit registers visible to the program
        
- **Protected mode features still active** (paging, protection, multitasking)
    
- **Each V86 task** thinks it owns the entire machine
    

### Protection and Translation

- **Page fault handling**: OS maps V86 memory accesses to physical pages
    
- **I/O trapping**: OS intercepts hardware access to virtualize devices
    
- **Interrupt handling**: OS catches real-mode interrupts and emulates them
    
- **Privilege level**: V86 tasks run at ring 3 (least privilege)
    

### Purpose

- **DOS boxes in Windows 3.x/9x**: Run DOS programs in windows
    
- **OS/2 compatibility**: Run multiple DOS applications concurrently
    
- **Early virtualization**: Foundation for later hardware virtualization
    

### The Problem It Solved

Without V86 mode, running a DOS program meant:

1. Rebooting into DOS
    
2. Running the program
    
3. Rebooting back to Windows
    

V86 mode enabled **instant switching** between DOS sessions and protected-mode OS.

### Modern Relevance

- **Almost obsolete**: 64-bit Windows removed V86 mode support
    
- **Legacy BIOS boot**: Still used briefly during boot on some systems
    
- **Virtualization replaced it**: VT-x/AMD-V do this better
    
- **DOSBox emulation**: Software emulation suffices for retro gaming
    

---

## 4. System Management Mode (386SL+ 1990)

### Historical Context (1990)

Laptops emerged, requiring power management. Intel needed a way to control hardware (CPU speed, fan, battery) without the operating system interfering.

### Motivation

- **Power management**: Throttle CPU, suspend/resume, battery monitoring
    
- **Hardware control**: Thermal throttling, fan speed control
    
- **OS transparency**: Work regardless of what OS is running (even crashed ones)
    
- **Vendor-specific features**: OEMs add custom functionality
    

### What Makes SMM Unique

**SMM is a "secret" mode**:

- **Entirely hidden from the OS** (no OS API to detect it)
    
- **No OS visibility** (OS can't see SMM code or data)
    
- **Highest privilege** (supersedes even ring 0)
    
- **Asynchronous entry** (triggered by System Management Interrupt - SMI)
    

### Entering and Exiting SMM

1. Hardware asserts SMI signal (power button, thermal event, chipset timer)
    
2. CPU saves entire state (registers, segment descriptors, etc.) to **SMRAM**
    
3. CPU switches to SMM (real-mode-like environment with 16-bit addressing)
    
4. Executes **SMI handler** (OEM/firmware code in SMRAM)
    
5. **RSM instruction** restores state and resumes normal operation
    
6. **OS has no idea anything happened**
    

### SMRAM (System Management RAM)

- **Special protected memory** (reserved by BIOS)
    
- **Only accessible in SMM** (OS can't read/write it)
    
- **Cache-as-RAM** technique used before DRAM initialization
    
- **Lockable** (can be hardware-protected after boot)
    

### Purpose

- **ACPI (Advanced Configuration and Power Interface)**: Modern power management
    
- **Thermal management**: Reduce CPU speed when overheating
    
- **Lid close/suspend**: Save system state to RAM or disk
    
- **Legacy emulation**: PS/2 keyboard/mouse emulation for USB
    
- **BIOS updates**: Flash new firmware from within OS
    

### The "Ring -2" Concept

Security researchers call SMM **"Ring -2"** (more privileged than ring 0):

- SMM can read/write all OS memory
    
- SMM can intercept/modify any system call
    
- SMM can persist across OS reinstalls (if malicious)
    
- **Rootkits have been implemented in SMM** (nearly undetectable)
    

### Modern Importance

- **Still very active** on all x86 CPUs (laptops, servers, desktops)
    
- **UEFI firmware uses SMM** extensively
    
- **Security nightmare** (SMM vulnerabilities are critical)
    
- **Intel SMI Transfer Monitor (STM)** attempts to virtualize SMM
    

---

## 5. Long Mode (x86_64 - 2003)

### Historical Context (2003 - AMD64)

By 2000, the 4GB address space of 32-bit x86 was becoming a bottleneck. Servers needed more RAM for databases, workstations needed larger files. Intel tried a clean break with Itanium (IA-64), which failed spectacularly because it broke backward compatibility.

### Motivation

- **64-bit addressing**: Support over 4GB RAM (16 exabytes theoretical)
    
- **More registers**: 16 general-purpose registers (up from 8)
    
- **Larger registers**: 64-bit operations (RAX, RBX, etc.)
    
- **Backward compatibility**: Run existing 32-bit/16-bit software
    
- **Avoid Itanium's mistake**: Don't break the ecosystem
    

### AMD's Brilliant Move

AMD extended x86 to 64-bit **while preserving backward compatibility**, unlike Intel's Itanium. The market chose AMD64, forcing Intel to adopt it (as Intel 64, formerly EM64T).

### Long Mode Has Two Sub-modes

#### 64-bit Mode

- **Pure 64-bit operation**
    
- **64-bit virtual addresses** (canonical form: bits 48-63 must match bit 47)
    
- **64-bit physical addresses** (hardware-dependent, typically 40-52 bits)
    
- **Flat memory model** (segmentation mostly disabled)
    
- **New RIP-relative addressing**
    
- **No V86 mode** (can't run real-mode programs directly)
    
- **Legacy segments (FS, GS) remain** (for thread-local storage)
    

#### Compatibility Mode

- **32-bit or 16-bit programs run under 64-bit OS**
    
- **Protected mode environment** (segmentation, paging still active)
    
- **32-bit registers and instructions** (programs don't know they're on 64-bit CPU)
    
- **Multiple 32-bit processes** can run simultaneously
    
- **Cannot enter V86 mode from compatibility mode**
    

### Key New Features

**Registers**:

- 16 GPRs (RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, R8-R15)
    
- All 16 registers are 64-bit (low 32 bits accessible as R8D, etc.)
    
- 16 SSE registers (XMM0-XMM15)
    
- RIP (Instruction Pointer) becomes 64-bit
    

**Paging Enhancements**:

- **4-level paging** (9-9-9-9-12 bit split)
    
- **52-bit physical addresses** (hardware dependent)
    
- **Execute Disable (NX) bit** (prevent code execution in data pages)
    
- **Global Pages** (don't flush from TLB on CR3 write)
    

**Segmentation Simplification**:

- **Segmentation largely disabled** (base always 0, limit ignored)
    
- **FS and GS remain** (used for kernel structures, thread-local storage)
    
- **CS, DS, ES, SS** act as flat segments (base 0, limit infinity)
    

### Purpose

- **Modern OS requirement**: Windows 10/11 x64, Linux x86_64, macOS
    
- **Large memory support**: Database servers, virtualization hosts
    
- **High-performance computing**: More registers = less spilling
    
- **Security features**: NX bit, SMEP, SMAP (built on 64-bit paging)
    
- **Future-proofing**: 16 exabyte virtual address space
    

### Why Long Mode Succeeded (and Itanium Failed)

|Factor|x86_64 (AMD64)|Itanium (IA-64)|
|---|---|---|
|**Backward compatibility**|Run 32-bit apps directly|Emulation (slow/buggy)|
|**Compiler support**|Minor changes to existing compilers|Entirely new optimization model|
|**Hardware complexity**|Incremental change|Completely new architecture|
|**Market timing**|Worked on existing x86 ecosystem|Required new everything|

---

## Mode Transitions and Relationships

### Boot Sequence (Modern System)

text

CPU Reset → Real Mode (16-bit, no protection)
    ↓
BIOS/UEFI executes (still in real mode)
    ↓
Switch to Protected Mode (32-bit)
    ↓
Optionally enter Virtual 8086 Mode (for legacy BIOS calls)
    ↓
Enter Long Mode (64-bit)
    ↓
Potentially enter SMM (anytime, transparently)

### Mode Characteristics Summary

|Mode|Address Size|Privilege Levels|Paging|Backward Compatibility|Primary Use|
|---|---|---|---|---|---|
|**Real**|20-bit (1MB)|1 (ring 0 only)|No|N/A|Boot, BIOS|
|**Protected (32-bit)**|32-bit (4GB)|4 rings|Optional|Real-mode via V86|Legacy 32-bit OS|
|**Virtual 8086**|20-bit (1MB)|Ring 3 (emulated)|Yes|Real-mode apps|DOS boxes|
|**System Management**|16/32/64-bit|Ring -2|Optional|None|Power mgmt, firmware|
|**Long (64-bit)**|64-bit (16EB)|4 rings (deprecated)|Required (4-level)|32/16-bit via compat|Modern OS|
|**Long (compat)**|32-bit (4GB)|4 rings|Required|32/16-bit apps|Legacy app support|

### The Hidden Complexity: Mode Switching Costs

Each mode switch has a **performance penalty**:

- **Real → Protected**: Setup GDT, IDT, enable PE bit (slow, but one-time)
    
- **Protected → Long**: Enable PAE, set LME bit, enable paging (medium)
    
- **Long → Protected**: Clear LME, disable paging, revert to segment registers (slow)
    
- **Entering SMM**: Hardware saves full CPU state (fast, but hidden)
    

### Why All These Modes Still Exist

**The x86 architecture carries its history in silicon**:

- Real mode remains for **boot compatibility** (BIOS, UEFI legacy boot)
    
- Protected mode for **32-bit OS support** (Windows XP, legacy servers)
    
- V86 mode for **DOS compatibility** (though mostly obsolete)
    
- SMM for **firmware power management** (critical for laptops)
    
- Long mode for **modern 64-bit computing**
    

This **evolutionary baggage** is both x86's greatest strength (backward compatibility) and weakness (immense architectural complexity). Newer architectures like ARM dropped legacy support, but x86 must maintain all these modes forever.