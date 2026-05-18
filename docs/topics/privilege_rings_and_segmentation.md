Here's pdf about segmentation which I downloaded from [here](https://gitlab.com/opensecuritytraining/arch2001_x86-64_os_internals_slides_and_subtitles/-/blob/master/04_SegmentationAndPrivilege/_Arch2001_04_SegmentationAndPrivilege_02_SegmentSelectorAndRegisters.pdf?ref_type=heads).

### Segments

![[_Arch2001_04_SegmentationAndPrivilege_02_SegmentSelectorAndRegisters.pdf]]

This gives nice explanation about Segments.

## Why Segmentation Existed

### The Original Problem (8086 era)

The 8086 had **20 address lines** (could address 1MB) but only **16-bit registers** (max 64KB directly). Intel needed a way to generate 20-bit addresses from 16-bit registers.

### The Solution: Segments

Instead of a single flat address space, memory was divided into **segments** - contiguous blocks of up to 64KB. Each address was specified as:

text

Physical Address = (Segment Selector × 16) + Offset

Where:

- **Segment Selector**: 16-bit value pointing to a segment (shifted left 4 bits)
    
- **Offset**: 0-65535 within that segment
    

This allowed programs to access 1MB using only 16-bit registers by switching between segments.

### The Problem With Segments

While clever, segmentation created **awkward programming models**:

- **Near vs far pointers**: Pointers within same segment vs different segments
    
- **64KB limits**: Data structures couldn't exceed 64KB without segment switching
    
- **Memory fragmentation**: Frequent segment allocation/deallocation
    
- **Complexity**: Programmers had to manage segment registers manually
    

---

## Protected Mode Segments (80286+)

When Intel introduced protected mode, they **enhanced segmentation** rather than replacing it:

### What Changed in Protected Mode

|Feature|Real Mode|Protected Mode|
|---|---|---|
|**Segment size**|Fixed 64KB|Variable (up to 4GB or 64KB)|
|**Segment base**|Selector × 16|Arbitrary 32-bit address|
|**Segment limit**|Implicit (64KB)|Explicit (up to 4GB)|
|**Protection**|None|Privilege levels, read/write/execute|
|**Access method**|Direct|Through descriptor tables|

### The Core Insight

In protected mode, **segment selectors are no longer addresses** - they're **indices into tables**. This abstraction allows the OS to place segments anywhere in memory and control access to them.

---

### GDT & LDT
![[_Arch2001_04_SegmentationAndPrivilege_03_GDT&LDT.pdf]]

### What Is the GDT?

The GDT is a **system-wide array of 8-byte (32-bit) or 16-byte (64-bit) segment descriptors**. It defines all memory segments available to all tasks on the system (except those in LDTs).

### GDT Structure and Location

The CPU uses a special register called **GDTR** (Global Descriptor Table Register) to locate the GDT:

```text
GDTR (64-bit) Structure:
┌──────────────┬─────────────────┐
│    Limit     │      Base       │
│   (16 bits)  │   (64 bits)     │
└──────────────┴─────────────────┘
```


- **Base**: Linear address where GDT starts
    
- **Limit**: Size of GDT in bytes minus 1 (maximum 65535 bytes = 8191 entries)
    

### Why 8191 Entries and Not 8192?

The **first entry (selector 0) is reserved as a null descriptor**. Loading a null selector into a segment register (CS, DS, ES, etc.) marks it as unused and generates an exception if accessed.

### Segment Descriptor Format (32-bit Protected Mode)

Each entry is 8 bytes (64 bits) with this structure:

```text

Bytes: 0     1     2     3     4     5     6     7
       +-----+-----+-----+-----+-----+-----+-----+-----+
       │ Limit 15:0  │ Base 15:0    │ Base 23:16    │
       +-----+-----+-----+-----+-----+-----+-----+-----+
       │ Access Byte │ Limit 19:16 │ Flags │ Base 31:24 │
       +-----+-----+-----+-----+-----+-----+-----+-----+
```


### Fields Explained

|Field|Size|Description|
|---|---|---|
|**Base (32 bits)**|32|Starting linear address of segment|
|**Limit (20 bits)**|20|Size of segment (units of 1 byte or 4KB)|
|**Access Byte**|8|Type, privilege level, present bit|
|**Flags**|4|Granularity, size, long mode indicators|

### The Access Byte (Most Critical Field)

```text

Bits: 7   6-5   4   3   2-1   0
     +---+-----+---+---+-----+---+
     │ P │ DPL │ S │ E │ Type│ A │
     +---+-----+---+---+-----+---+

```


|Bit|Name|Meaning|
|---|---|---|
|**P (Present)**|1|Segment present in memory (0 = not present, #NP exception)|
|**DPL (Descriptor Privilege Level)**|2|Minimum privilege level to access (0-3)|
|**S (Descriptor Type)**|1|0 = System, 1 = Code/Data|
|**E (Executable)**|1|0 = Data segment, 1 = Code segment|
|**Type**|2-3|Further type details (read/write, conforming, etc.)|
|**A (Accessed)**|1|Set by CPU when segment is accessed (for paging to disk)|

### Granularity Flag (G)

- **G=0**: Limit is in 1-byte units (max segment size = 1MB)
    
- **G=1**: Limit is in 4KB pages (max segment size = 4GB)
    

This allows 20 bits of limit to represent either 1MB or 4GB.

### What Is the LDT?

The LDT provides **process-specific segments**, allowing each task to have its own set of segments without cluttering the global GDT.

### How LDT Works

1. **One LDT per task** (optionally)
    
2. **LDT size**: Up to 8192 entries (64KB)
    
3. **LDT location**: Pointed to by LDTR (LDT Register)
    
4. **LDT Descriptor**: Stored in GDT (type = 0x82 for 32-bit, 0x92 for 64-bit)
    

### The LDTR Register

Like GDTR, LDTR has two parts:

- **Visible part**: Selector pointing to LDT descriptor in GDT
    
- **Hidden part**: Cached LDT base address, limit, and access rights
    

### Loading the LDT

```text

1. Create LDT descriptor in GDT (type = LDT)
2. Load selector into LDTR using LLDT instruction (privileged)
3. CPU reads GDT entry, loads hidden LDTR with base/limit
4. Now selectors with TI=1 access this LDT

```

### LDT vs GDT: When to Use Which

|Aspect|GDT|LDT|
|---|---|---|
|**Scope**|System-wide|Per-process|
|**Number**|Exactly 1|Many (one per task)|
|**Typical content**|OS segments, TSS|Process-specific data|
|**Typical use**|Kernel code/data|Application segments|
|**Selector TI bit**|0|1|

### Why LDT Fell Out of Favor

Modern OSes (Linux, Windows) **don't use LDT** because:

1. **Paging handles per-process isolation better**
    
2. **LDT adds complexity** (LDT switching on context switch)
    
3. **Limited slots** (8192 max, but GDT is only 256 on many systems)
    
4. **Performance overhead** (extra memory indirection)
    

However, **Wine/Proton still uses LDT** to emulate Windows segment selectors for 16-bit and 32-bit Windows applications.

---

### Segment Descriptor

![[_Arch2001_04_SegmentationAndPrivilege_04_SegmentDescriptors.pdf]]

### What Is a Selector?

A **selector** is a 16-bit value stored in segment registers (CS, DS, SS, ES, FS, GS) that points to a GDT or LDT entry.

### Selector Format

```text

Bits: 15-3     2      1-0
     +--------+------+-----+
     │ Index  │ TI   │ RPL │
     +--------+------+-----+
```


|Field|Bits|Meaning|
|---|---|---|
|**Index**|13|Index into descriptor table (0-8191)|
|**TI (Table Indicator)**|1|0 = GDT, 1 = LDT|
|**RPL (Requested Privilege Level)**|2|Requested access privilege (0-3)|

### The Null Selector

Selector value 0 (index 0, TI=0, RPL=0) is the **null selector**:

- Points to the invalid first GDT entry
    
- Loading into CS causes #GP (cannot execute null code)
    
- Loading into DS/ES/FS/GS marks segment as unusable
    
- Used to initialize unused segment registers
    

### Loading Selectors

Different instructions load different segment registers:

- **CS**: Far jump, far call, interrupt return (IRET), task switch
    
- **SS**: Load SS instruction (requires privilege checks)
    
- **DS/ES/FS/GS**: MOV to segment register, POP to segment register
    

---

## Code and Data Segment Types

### Data Segment Types (E=0 in Access Byte)

|Type Value|Name|Description|
|---|---|---|
|0000 (0)|Read-Only|Cannot write|
|0001 (1)|Read-Only, Accessed|Read-only, accessed flag set|
|0010 (2)|Read/Write|Can read and write|
|0011 (3)|Read/Write, Accessed|Read/write, accessed flag set|
|0100 (4)|Read-Only, Expand-Down|Grows downward (stacks)|
|0101 (5)|Read-Only, Expand-Down, Accessed|Expand-down with accessed flag|
|0110 (6)|Read/Write, Expand-Down|Stack segment (grows down)|
|0111 (7)|Read/Write, Expand-Down, Accessed|Stack with accessed flag|

**Expand-Down Explanation**:

- **Normal segment**: Access from base to base+limit
    
- **Expand-down segment**: Access from base+limit to top of segment (stacks)
    

### Code Segment Types (E=1 in Access Byte)

|Type Value|Name|Description|
|---|---|---|
|1000 (8)|Execute-Only|Cannot read, only execute|
|1001 (9)|Execute-Only, Accessed|Execute-only with accessed flag|
|1010 (10)|Execute/Read|Can execute and read|
|1011 (11)|Execute/Read, Accessed|Execute/read with accessed flag|
|1100 (12)|Execute-Only, Conforming|Same privilege execution|
|1101 (13)|Execute-Only, Conforming, Accessed|Conforming with accessed flag|
|1110 (14)|Execute/Read, Conforming|Conforming with read permission|
|1111 (15)|Execute/Read, Conforming, Accessed|Full access with accessed flag|

**Conforming Explanation**:

- **Non-conforming**: Transfers to code segment require equal privilege
    
- **Conforming**: Can transfer from any privilege level, keeps current privilege

---

### Call Gates

![[_Arch2001_04_SegmentationAndPrivilege_06_CallGates.pdf]]

### What Is a Call Gate?

A **call gate** is a special type of descriptor (S=0, Type=0xC for 32-bit, Type=0xC for 64-bit) that **controls transitions between privilege levels**. It's the original x86 mechanism for user code to call kernel functions.

### Why Call Gates Existed

Before `syscall`/`sysenter` instructions, call gates provided:

- **Controlled entry points** into higher privilege code
    
- **Automatic privilege checking**
    
- **Parameter copying** (up to 32 bytes) between stacks
    
- **Far call/call gate mechanism** (user uses far call to gate)
    

### Call Gate Descriptor Format (64-bit)

```text

Bytes: 0     1     2     3     4     5     6     7     8     9     10    11    12    13    14    15
       +-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+
       │ Offset 15:0 │ Segment Selector │ Count │  Type  │0│ DPL│ P │ Offset 31:16 │   Offset 63:32    │
       +-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+-----+


```

### Key Fields

|Field|Meaning|
|---|---|
|**Segment Selector**|Target code segment to execute|
|**Offset**|Entry point within target segment|
|**Count (Param Count)**|Number of parameters to copy (0-31, 32-bit only)|
|**Type**|Must be 0xC (call gate)|
|**DPL**|Minimum privilege to use this gate|

### How a Call Gate Works

```text

User Mode (Ring 3):
    FAR CALL [call_gate_selector]
                    ↓
CPU Check: CPL ≤ Gate.DPL? (User must have authority)
CPU Check: Gate.DPL ≤ Target Segment.DPL? (Gate must be between)
                    ↓
CPU saves SS, ESP, CS, EIP (current stack)
                    ↓
CPU loads new SS, ESP from TSS (kernel stack)
                    ↓
CPU copies parameters (0-31 dwords) from user to kernel stack
                    ↓
CPU loads CS:EIP from call gate descriptor
                    ↓
CPU jumps to kernel handler at Target Segment:Offset
                    ↓
Kernel executes (now at Ring 0)
                    ↓
FAR RET instruction returns, copying parameters back
```

### Why Call Gates Are Obsolete

Modern OSes use faster mechanisms:

- **`sysenter`/`sysexit`** (Intel, 2002): Faster than call gates
    
- **`syscall`/`sysret`** (AMD, 2003): Even faster, single instruction
    
- **No parameter copying overhead** (registers pass parameters)
    
- **Simpler setup** (no gate descriptors in GDT/LDT)
    

However, **call gates remain** for legacy OS compatibility.

---

## Part 8: Segmentation in Long Mode (x86_64)

### The Great Change

When AMD designed x86_64, they **deprecated most segmentation** but couldn't remove it entirely (backward compatibility).

### What Changed

|Aspect|32-bit Protected Mode|64-bit Long Mode|
|---|---|---|
|**Segment base**|Arbitrary 32-bit|**Force to 0**|
|**Segment limit**|Variable (up to 4GB)|**Force to infinity**|
|**Segmentation usefulness**|Memory management, protection|**Almost useless**|
|**Paging requirement**|Optional|**Required**|

### What Remains of Segmentation in 64-bit

Despite flat segmentation, several segment registers still work:

**CS (Code Segment)**:

- **Depends on CS.L flag** (1 = 64-bit, 0 = 32-bit compatibility)
    
- **Depends on CS.D flag** (0 = default 16/32-bit)
    
- **Privilege level (CPL) still tracked** (bits 0-1 of CS)
    

**SS (Stack Segment)**:

- **Base forced to 0**
    
- **Limit forced to infinity**
    
- Only privilege level matters
    

**DS, ES, SS**:

- **Load non-null selectors cause #GP?** Actually, they ignore base/limit
    
- Must point to valid descriptors, but descriptors' base/limit ignored
    
- Only DPL and present bit checked
    

**FS and GS**:

- **Full segmentation still works!** (base can be non-zero)
    
- Used for **thread-local storage** (TLS)
    
- `WRFSBASE`/`WRGSBASE` instructions (if available) set base directly
    
- `SWAPGS` instruction exchanges GS base with kernel GS base (syscall)
    

### The FS/GS Exception

Why keep FS/GS when all other segments are flat?

- **Thread-local storage**: Each thread needs different base address
    
- **Kernel per-CPU data**: Each CPU has private data area
    
- **Performance**: Base register + offset is faster than pointer chasing
    

---

## TSS (Task State Segment)

### What Is the TSS?

The TSS is a **special segment** (type = 0x89 for 32-bit, 0x89 for 64-bit) that contains:

- **Stack pointers** (SS0:ESP0, SS1:ESP1, SS2:ESP2) for privilege transitions
    
- **I/O Map Base Address** (for I/O permission bitmap)
    
- **IST pointers** (x86_64: Interrupt Stack Table, 7 stacks)
    
- **Previous TSS link** (for hardware task switching)
    

### TSS in 32-bit vs 64-bit

|Feature|32-bit Protected Mode|64-bit Long Mode|
|---|---|---|
|**Required for**|Task switching, stack switching|Stack switching (IST)|
|**Hardware task switching**|Yes (TSS link)|**Removed**|
|**Software task switching**|Optional|Required|
|**IST field**|No|Yes (7 stacks)|

### The TSS's Critical Role

Even though hardware task switching is deprecated, **TSS remains mandatory** because:

1. **Privilege level transitions** (ring 3 → ring 0 need Ring 0 stack pointer)
    
2. **Interrupt handling** (IST needs dedicated stacks)
    
3. **I/O permission bitmap** (control port access per task)
    

### TR Register (Task Register)

The **TR** register holds the **selector pointing to TSS descriptor** in GDT. `LTR` instruction loads TR (privileged, done once during OS initialization).

---

## Putting It All Together

### Typical GDT Layout (Modern x86_64 Kernel)

```
text

Index | Selector | Type               | Purpose
------|----------|--------------------|----------------------------------
0     | 0x00     | Null               | Unused segment (required)
1     | 0x08     | Kernel Code (64-bit) | Executing kernel code
2     | 0x10     | Kernel Data         | Data access in kernel
3     | 0x18     | User Code (64-bit)  | User applications (if using segments)
4     | 0x20     | User Data           | User data (if using segments)
5     | 0x28     | TSS (64-bit)        | Stack switching, IST
6     | 0x30     | (Optional LDT)      | Per-process segments (rare)
```

### Why Modern OSes Ignore Most Segmentation

**Linux and Windows use a "flat model"**:

- Only **4 segments**: Kernel/User, Code/Data
    
- **Base = 0, Limit = infinity**
    
- **Paging handles all protection**
    
- Segmentation only provides **privilege level tracking**
    

### What Segmentation Still Gives Us

|Feature|Provided by|
|---|---|
|**Privilege levels (rings)**|CS/SS DPL fields|
|**Stack switching**|TSS (via SS0:ESP0 in TSS)|
|**Interrupt stack switching**|TSS (IST fields)|
|**Thread-local storage**|FS/GS base registers|
|**I/O permissions**|TSS (I/O bitmap)|
|**System call entry**|CPU dispatches via gate or syscall instruction|

---

## Part 11: Summary of Evolution

### The Grand Arc of x86 Segmentation

```text

8086 (1978):
    Segments mandatory (64KB chunks)
        ↓
80286 (1982):
    Enhanced segmentation (GDT/LDT, protection)
        ↓
80386 (1985):
    Paging added (segments optional)
        ↓
Pentium Pro (1995):
    Flat model popular (segments deprecated)
        ↓
x86_64 (2003):
    Most segments forced flat (FS/GS remain)
```

### Why Segmentation Persists

Despite being largely bypassed, segmentation remains because:

1. **CPL must come from somewhere** (CS.RPL)
    
2. **Interrupt handling needs stacks** (TSS required)
    
3. **FS/GS are too useful to remove** (TLS, per-CPU data)
    
4. **Legacy OS compatibility** (Windows 10 still uses segments for subsystems)
    

### The Final Irony

The feature designed to **manage memory** (segmentation) was replaced by **paging**, which does it better. But the feature designed to **manage protection** (segmentation's DPL) remains embedded in the CPU because:

**Every instruction fetch checks CS.DPL. Every data access checks DS.DPL. Every interrupt checks IDT.DPL. You cannot escape segmentation entirely.**

It's like the foundation of an old house you don't see it, but the entire building rests on it.

---

