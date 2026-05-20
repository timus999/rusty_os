
---

## Control Registers (CR0-CR15) - The CPU's Configuration Switches

Control registers configure the CPU's most fundamental behaviors. You can think of them as the **master switches** for CPU features.

### CR0 - The Master Control Register

CR0 controls basic CPU operating modes and features. Think of it as the **power panel** for your CPU.

|Bit|Name|What It Controls|Why You Care|
|---|---|---|---|
|**0**|PE (Protection Enable)|0 = Real mode, 1 = Protected mode|Switching to protected mode|
|**1**|MP (Monitor Coprocessor)|Controls FPU/math coprocessor interaction|Rarely touched|
|**2**|EM (Emulate Coprocessor)|1 = Emulate FPU in software (no hardware FPU)|Older CPUs|
|**3**|TS (Task Switched)|Set on task switch to save FPU state|Context switching|
|**4**|ET (Extension Type)|386/486 FPU identification|Obsolete|
|**5**|NE (Numeric Error)|1 = Enable native FPU error reporting|FPU exception handling|
|**16**|WP (Write Protect)|1 = Kernel can't write read-only user pages|Copy-on-write protection|
|**18**|AM (Alignment Mask)|1 = Enable alignment checking (if AC flag also set)|Detecting misaligned access|
|**29**|NW (Not Write-Through)|Cache control (rarely used)|Memory type management|
|**30**|CD (Cache Disable)|1 = Disable caching entirely|Debugging, benchmarking|
|**31**|PG (Paging Enable)|0 = No paging, 1 = Enable paging|**Essential for long mode**|

**The Critical Bits for OS Development**:

|Bit Sequence|Mode|
|---|---|
|PE=0, PG=0|Real mode (16-bit, no protection)|
|PE=1, PG=0|Protected mode (segmentation only, no paging)|
|PE=1, PG=1|Paging enabled (normal modern OS mode)|

### CR2 - The Page Fault Address Register

CR2 is **read-only** and has only one job: store the virtual address that caused the last page fault.

text

When page fault occurs:
CR2 = 0x00007FFFFF123456  (the address that caused the fault)

Page fault handler reads CR2 to know which address to fix:

- **Demand paging**: CR2 tells you which page to load
    
- **Copy-on-write**: CR2 tells you which page to duplicate
    
- **Invalid access**: CR2 tells you which address to report
    

### CR3 - The Page Table Root (Most Important for Paging)

CR3 holds the **physical address of the PML4 table** (top-level page table).

```text

Bits 0-11:  Reserved/control (PCID)
Bits 12-51: Physical address of PML4 (page-aligned)
Bits 52-63: Reserved (must be 0)
```


**Why CR3 matters**:

- **Change CR3** = switch to entirely different address space (process context switch)
    
- **Read CR3** = discover current page tables (debugging)
    
- **Write CR3** = flush TLB (force reload page tables)
    

**Context Switching Flow**:

```text

Process A running:
    CR3 = 0x1000 (PML4 of Process A)
    
[Interrupt to scheduler]
    
Process B scheduled:
    CR3 = 0x8000 (PML4 of Process B)  ← CPU now uses Process B's mappings
```

### CR4 - Feature Control Register

CR4 enables/disables advanced CPU features. Think of it as the **feature switchboard**.

|Bit|Name|What It Enables|
|---|---|---|
|**5**|PAE (Physical Address Extension)|36/40/52-bit physical addresses (required for long mode)|
|**7**|PGE (Page Global Enable)|Global pages (G bit in PTEs)|
|**8**|PCE (Performance Monitoring Counter)|RDPMC instruction (performance profiling)|
|**9**|OSFXSR (OS Support for FXSAVE/FXRSTOR)|SSE register saving|
|**10**|OSXMMEXCPT (OS Support for SIMD Exceptions)|SSE exception handling|
|**13**|VMXE (VMX Enable)|Hardware virtualization (VT-x)|
|**17**|PCIDE (PCID Enable)|Process Context Identifiers (TLB tagging)|
|**18**|OSXSAVE (XSAVE Enable)|Advanced state saving (AVX, etc.)|
|**20**|SMEP (Supervisor Mode Execution Prevention)|Prevent kernel from executing user pages|
|**21**|SMAP (Supervisor Mode Access Prevention)|Prevent kernel from accessing user pages|
|**22**|PKE (Protection Key Enable)|Memory protection keys (thread safety)|

**Critical for long mode**: PAE (bit 5) must be set BEFORE enabling paging.

### CR8 - Task Priority Register (x86_64 only)

CR8 is a 4-bit register (bits 0-3) that masks interrupts based on priority:

```text

CR8 value: 0-15 (higher number = higher priority)
If interrupt priority ≤ CR8 → interrupt blocked
If interrupt priority > CR8 → interrupt can fire
```

Used by OS schedulers to prevent low-priority interrupts from interrupting high-priority work.

### Extended Control Registers (CR1, CR5-CR7, CR9-CR15)

These are **reserved** for future use. Writing to them causes #GP (General Protection Fault).

## Quick Reference Table: CR Register Summary

|Register|Purpose|Key Bits|When Modified|
|---|---|---|---|
|**CR0**|CPU mode control|PE (0), PG (31)|Boot (once)|
|**CR2**|Page fault address|All 64 bits|Read-only (CPU writes)|
|**CR3**|Page table root|Bits 12-51 (PML4 address)|Every context switch|
|**CR4**|Feature enable|PAE (5), PCIDE (17), SMEP (20)|Boot (once per feature)|
|**CR8**|Interrupt priority|Bits 0-3|On every interrupt|


---
