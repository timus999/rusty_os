(May 20, 2026)

---

This is the Day 14 in my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

After learning two memory protection techniques: segmentation and paging. While the former uses variable-sized memory regions and suffers from external fragmentation, the latter uses fixed-sized pages and allows much more fine-grained control over access permissions.

Paging stores the mapping information for pages in page tables with one or more levels. The x86_64 architecture uses 4-level page tables and a page size of 4 KiB. The hardware automatically walks the page tables and caches the resulting translations in the translation lookaside buffer (TLB). This buffer is not updated transparently and needs to be flushed manually on page table changes.

---

### Page Faults

 **Our kernel already runs on paging**. The bootloader that I added in [[Day2]] has already set up a 4-level paging hierarchy that maps every page of the kernel to a physical frame. The bootloader does this because paging is mandatory in 64-bit mode on x86_64.

This means that every memory address that I used in my kernel was a virtual address. Accessing the VGA buffer at address `0xb8000` only worked because the bootloader _identity mapped_ that memory page, which means that it mapped the virtual page `0xb8000` to the physical frame `0xb8000`.

Paging makes the kernel already relatively safe, since every memory access that is out of bounds causes a page fault exception instead of writing to random physical memory. The bootloader even sets the correct access permissions for each page, which means that only the pages containing code are executable and only data pages are writable.

I tried to cause a page fault by accessing some memory outside of the kernel. First, I created a page fault handler and registered it in the IDT, so that I see a page fault exception instead of a generic double fault:

```rust
// in src/interrupts.rs

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        […]

        idt.page_fault.set_handler_fn(page_fault_handler); // new

        idt
    };
}

use x86_64::structures::idt::PageFaultErrorCode;
use crate::hlt_loop;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}
```

The [`CR2`](https://en.wikipedia.org/wiki/Control_register#CR2) register is automatically set by the CPU on a page fault and contains the accessed virtual address that caused the page fault. I used the [`Cr2::read`](https://docs.rs/x86_64/0.14.2/x86_64/registers/control/struct.Cr2.html#method.read) function of the `x86_64` crate to read and print it. The [`PageFaultErrorCode`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.PageFaultErrorCode.html) type provides more information about the type of memory access that caused the page fault, for example, whether it was caused by a read or write operation. For this reason, I printed it too. I can’t continue execution without resolving the page fault, so I entered a [`hlt_loop`](https://os.phil-opp.com/hardware-interrupts/#the-hlt-instruction) at the end.

Then I tried to access some memory outside the kernel:

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    rusty_os::init();

    // new
    let ptr = 0xdeadbeaf as *mut u8;
    unsafe { *ptr = 42; }

    // as before
    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    rusty_os::hlt_loop();
}
```

When I ran it, I saw that the page fault handler was called:
![[Screenshot From 2026-05-20 10-36-21.png]]

The `CR2` register indeed contained `0xdeadbeaf`, the address that I tried to access. The error code told me through the [`CAUSED_BY_WRITE`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.PageFaultErrorCode.html#associatedconstant.CAUSED_BY_WRITE) that the fault occurred while trying to perform a write operation. It told me even more through the [bits that are _not_ set](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.PageFaultErrorCode.html). For example, the fact that the `PROTECTION_VIOLATION` flag is not set means that the page fault occurred because the target page wasn’t present.

I saw that the current instruction pointer is `0x2031b2`, so I knew that this address points to a code page. Code pages are mapped read-only by the bootloader, so reading from this address works but writing causes a page fault. I tried this by changing the `0xdeadbeaf` pointer to `0x205156`:

```rust
// Note: The actual address might be different for you. Use the address that
// your page fault handler reports.
let ptr = 0x2031b2 as *mut u8;

// read from a code page
unsafe { let x = *ptr; }
println!("read worked");

// write to a code page
unsafe { *ptr = 42; }
println!("write worked");
```

By commenting out the last line, I saw that the read access works, but the write access caused a page fault:
![[Screenshot From 2026-05-20 10-35-47.png]]
I saw that the _“read worked”_ message is printed, which indicates that the read operation did not cause any errors. However, instead of the _“write worked”_ message, a page fault occurred. This time the [`PROTECTION_VIOLATION`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.PageFaultErrorCode.html#associatedconstant.PROTECTION_VIOLATION) flag is set in addition to the [`CAUSED_BY_WRITE`](https://docs.rs/x86_64/0.14.2/x86_64/structures/idt/struct.PageFaultErrorCode.html#associatedconstant.CAUSED_BY_WRITE) flag, which indicates that the page was present, but the operation was not allowed on it. In this case, writes to the page are not allowed since code pages are mapped as read-only.

### Accessing the Page Tables

I tried to take a look at the page tables that define how the kernel is mapped:

```rust
// in src/main.rs

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    blog_os::init();

    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!("Level 4 page table at: {:?}", level_4_page_table.start_address());

    […] // test_main(), println(…), and hlt_loop()
}
```

The [`Cr3::read`](https://docs.rs/x86_64/0.14.2/x86_64/registers/control/struct.Cr3.html#method.read) function of the `x86_64` returns the currently active level 4 page table from the `CR3` register. It returns a tuple of a [`PhysFrame`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/frame/struct.PhysFrame.html) and a [`Cr3Flags`](https://docs.rs/x86_64/0.14.2/x86_64/registers/control/struct.Cr3Flags.html) type. I was only interested in the frame, so I ignored the second element of the tuple.

When I ran it, I got the following output:

![[Screenshot From 2026-05-20 10-38-59.png]]
So the currently active level 4 page table is stored at address `0x1000` in _physical_ memory, as indicated by the [`PhysAddr`](https://docs.rs/x86_64/0.14.2/x86_64/addr/struct.PhysAddr.html) wrapper type. The question now was: how can I access this table from the kernel?

Accessing physical memory directly is not possible when paging is active, since programs could easily circumvent memory protection and access the memory of other programs otherwise. So the only way to access the table is through some virtual page that is mapped to the physical frame at address `0x1000`. This problem of creating mappings for page table frames is a general problem since the kernel needs to access the page tables regularly, for example, when allocating a stack for a new thread.

So I will try to implement this tomorrow.

---


I learned that my kernel already runs on top of paging and that illegal memory accesses cause page fault exceptions. I tried to access the currently active page tables, but I wasn’t able to do it because the CR3 register stores a physical address that we can’t access directly from our kernel.

---
