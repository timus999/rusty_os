### Paging Mode Evolution

|Mode|Year|Virtual Bits|Max Physical|Page Size Options|Requires|
|---|---|---|---|---|---|
|**32-bit Paging**|1985|32-bit|4 GB|4 KB|[CR0.PG](https://cr0.pg/)=1|
|**PAE Paging**|1995|32-bit|64 GB|4 KB, 2 MB|CR4.PAE=1|
|**Long Mode (4-level)**|2003|48-bit|52-bit|4 KB, 2 MB, 1 GB|[CR0.PG](https://cr0.pg/)=1, CR4.PAE=1, EFER.LME=1|
|**5-level Paging**|2019|57-bit|52-bit|Same as 4-level|[CR0.PG](https://cr0.pg/)=1, CR4.PAE=1, CR4.LA57=1, EFER.LME=1|

### Mode 1: 32-bit Paging (Legacy)

**Structure**: 2-level page tables

- **Page Directory** (1024 entries × 4 bytes = 4KB)
    
- **Page Table** (1024 entries × 4 bytes = 4KB)
    

**Address Split**: 10-10-12 bits

- Bits 31-22: Directory index (10 bits)
    
- Bits 21-12: Table index (10 bits)
    
- Bits 11-0: Page offset (12 bits)
    

**Limitations**: Only 32-bit virtual, 32-bit physical (4GB max RAM)

### Mode 2: PAE Paging (Physical Address Extension)

**The Problem**: Servers needed >4GB RAM, but 32-bit CPUs still dominated.

**The Solution**: PAE added 4 more address lines (36-bit physical = 64GB).

**Structure**: 3-level page tables (but still 32-bit virtual)

- **PDPT** (Page Directory Pointer Table): 4 entries × 8 bytes = 32 bytes
    
- **Page Directory**: 512 entries × 8 bytes = 4KB
    
- **Page Table**: 512 entries × 8 bytes = 4KB
    

**Address Split**: 2-9-9-12 bits

- Bits 31-30: PDPT index (2 bits → 4 entries)
    
- Bits 29-21: Directory index (9 bits → 512 entries)
    
- Bits 20-12: Table index (9 bits → 512 entries)
    
- Bits 11-0: Page offset (12 bits)
    

**Why PAE is required for long mode**: Long mode uses the 64-bit version of PAE's 3-level structure (extended to 4 levels for 48-bit virtual).

### Mode 3: Long Mode Paging (The One You Need)

This is the **4-level paging** I explained earlier. It builds on PAE.

**Structure**: 4-level page tables (each entry 8 bytes)

- **PML4**: 512 entries (9 bits)
    
- **PDPT**: 512 entries (9 bits)
    
- **Page Directory**: 512 entries (9 bits)
    
- **Page Table**: 512 entries (9 bits)
    

**Entry Size Chain**: All 8 bytes × 512 entries = 4KB per table

**Virtual Address Split (48-bit)**:

```text

47-39    38-30    29-21    20-12    11-0
PML4     PDPT     PD       PT       Offset
(9 bits) (9 bits) (9 bits) (9 bits) (12 bits)
```

**Physical Address**: Up to 52 bits (stored in bits 12-51 of PTEs)

### Mode 4: 5-Level Paging (The Future)

**New Register Bit**: CR4.LA57 = 1 (Linear Address 57-bit)

**Structure**: 5-level page tables (adds PML5 above PML4)

- **PML5**: 512 entries (9 bits)
    
- **PML4**: 512 entries (9 bits)
    
- **PDPT**: 512 entries (9 bits)
    
- **PD**: 512 entries (9 bits)
    
- **PT**: 512 entries (9 bits)
    

**Virtual Address Split (57-bit)**:

```text

56-48    47-39    38-30    29-21    20-12    11-0
PML5     PML4     PDPT     PD       PT       Offset
(9 bits) (9 bits) (9 bits) (9 bits) (9 bits) (12 bits)
```

**Support**: Ice Lake and newer (check CPUID)

---

## Putting It All Together: Long Mode Paging Setup

Here's the sequence to enable long mode paging (conceptually):

### Step 1: Set Up Page Tables

```text

Create PML4 table (4KB, all zeros)
Create PDPT table (4KB, all zeros)
Create PD table (4KB, all zeros)
Create PT table (4KB, all zeros)
Fill PT with actual page mappings
Fill PD with entry pointing to PT
Fill PDPT with entry pointing to PD
Fill PML4 with entry pointing to PDPT
```

### Step 2: Configure CPU Registers

```text

CR3 = physical_address_of_PML4  (point to page tables)
CR4.PAE = 1                      (enable PAE - required)
CR4.PGE = 1                      (enable global pages - optional)
IA32_EFER.LME = 1                (enable long mode - MSR register)
CR0.PG = 1                       (enable paging - DO LAST)
CR0.WP = 1                       (write protect kernel pages)
```

### Step 3: Verify

```text

Now CPU is in 64-bit long mode with 4-level paging!
CR0.PG = 1, CR4.PAE = 1, EFER.LME = 1
```

---
