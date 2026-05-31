(May 21, 2026)

---

This is the Day 15 in my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

I first learned the concepts that there many [page tables accessing strategies](topics/Page_table_Access_Strategies).
After learning the theory, I started to implemented page table translation.

---

### Bootloader Support

All of these approaches that I learned require page table modifications for their setup. For example, mappings for the physical memory need to be created or an entry of the level 4 table needs to be mapped recursively. The problem is that I can’t create these required mappings without an existing way to access the page tables.

This means that I need the help of the bootloader, which creates the page tables that the kernel runs on. The bootloader has access to the page tables, so it can create any mappings that I need. In its current implementation, the `bootloader` crate has support for two of the above approaches, controlled through [cargo features](https://doc.rust-lang.org/cargo/reference/features.html#the-features-section):

- The `map_physical_memory` feature maps the complete physical memory somewhere into the virtual address space. Thus, the kernel has access to all physical memory and can follow the [_Map the Complete Physical Memory_](https://os.phil-opp.com/paging-implementation/#map-the-complete-physical-memory) approach.
- With the `recursive_page_table` feature, the bootloader maps an entry of the level 4 page table recursively. This allows the kernel to access the page tables.

I chose the first approach for my kernel since it is simple, platform-independent, and more powerful (it also allows access to non-page-table-frames). To enable the required bootloader support, I added the `map_physical_memory` feature to the `bootloader` dependency:

```toml
[dependencies]
bootloader = { version = "0.9", features = ["map_physical_memory"]}
```

With this feature enabled, the bootloader maps the complete physical memory to some unused virtual address range. To communicate the virtual address range to the kernel, the bootloader passes a _boot information_ structure.

---
### Boot Information

The `bootloader` crate defines a [`BootInfo`](https://docs.rs/bootloader/0.9/bootloader/bootinfo/struct.BootInfo.html) struct that contains all the information it passes to the kernel. With the `map_physical_memory` feature enabled, it currently has the two fields `memory_map` and `physical_memory_offset`:

- The `memory_map` field contains an overview of the available physical memory. This tells the kernel how much physical memory is available in the system and which memory regions are reserved for devices such as the VGA hardware. The memory map can be queried from the BIOS or UEFI firmware, but only very early in the boot process. For this reason, it must be provided by the bootloader because there is no way for the kernel to retrieve it later.
- The `physical_memory_offset` tells us the virtual start address of the physical memory mapping. By adding this offset to a physical address, we get the corresponding virtual address. This allows us to access arbitrary physical memory from the kernel.
- This physical memory offset can be customized by adding a `[package.metadata.bootloader]` table in Cargo.toml and setting the field `physical-memory-offset = "0x0000f00000000000"` (or any other value). However, note that the bootloader can panic if it runs into physical address values that start to overlap with the space beyond the offset, i.e., areas it would have previously mapped to some other early physical addresses. So in general, the higher the value (> 1 TiB), the better.

The bootloader passes the `BootInfo` struct to the kernel in the form of a `&'static BootInfo` argument to the `_start` function. I didn’t have this argument declared in the function yet, so I added it:

```rust
// in src/main.rs

use bootloader::BootInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! { // new argument
    […]
}
```

It wasn’t a problem to leave off this argument before because the x86_64 calling convention passes the first argument in a CPU register. Thus, the argument is simply ignored when it isn’t declared. However, it would be a problem if I accidentally used a wrong argument type, since the compiler doesn’t know the correct type signature of the entry point function.

### The `entry_point` Macro

Since the `_start` function is called externally from the bootloader, no checking of the function signature occurs. This means that I could let it take arbitrary arguments without any compilation errors, but it would fail or cause undefined behavior at runtime.

To make sure that the entry point function always has the correct signature that the bootloader expects, the `bootloader` crate provides an [`entry_point`](https://docs.rs/bootloader/0.6.4/bootloader/macro.entry_point.html) macro that provides a type-checked way to define a Rust function as the entry point. So I rewrote the entry point function to use this macro:

```rust
// in src/main.rs

use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    […]
}
```

I no longer needed to use `extern "C"` or `no_mangle` for the entry point, as the macro defines the real lower level `_start` entry point for me. The `kernel_main` function is now a completely normal Rust function, so I can choose an arbitrary name for it. The important thing is that it is type-checked so that a compilation error occurs when I use a wrong function signature, for example by adding an argument or changing the argument type.

Then I performed the same change in the `lib.rs`:

```rust
// in src/lib.rs

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    // like before
    init();
    test_main();
    hlt_loop();
}
```

Since the entry point is only used in test mode, I added the `#[cfg(test)]` attribute to all items. I gave the test entry point the distinct name `test_kernel_main` to avoid confusion with the `kernel_main` of the `main.rs`. We didn’t use the `BootInfo` parameter for now, so I prefixed the parameter name with a `_` to silence the unused variable warning.

--- 

### Implementation

Now that I have access to physical memory, I can finally start to implement the page table code. First, I took a look at the currently active page tables that the kernel runs on. In the second step, we created a translation function that returns the physical address that a given virtual address is mapped to. As a last step, we tried to modify the page tables in order to create a new mapping.

Before I began, I created a new `memory` module for my code:

```rust
// in src/lib.rs

pub mod memory;
```

For the module, I created an empty `src/memory.rs` file.

### Accessing the Page Tables

Yesterday, I tried to take a look at the page tables the kernel runs on, but failed since I couldn’t access the physical frame that the `CR3` register points to. I was now able to continue from there by creating an `active_level_4_table` function that returns a reference to the active level 4 page table:

```rust
// in src/memory.rs

use x86_64::{
    structures::paging::PageTable,
    VirtAddr,
};

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}
```

First, I read the physical frame of the active level 4 table from the `CR3` register. I then took its physical start address, convert it to a `u64`, and added it to `physical_memory_offset` to get the virtual address where the page table frame is mapped. Finally, I converted the virtual address to a `*mut PageTable` raw pointer through the `as_mut_ptr` method and then unsafely created a `&mut PageTable` reference from it. I created a `&mut` reference instead of a `&` reference because I will mutate the page tables later.

Then I used this function to print the entries of the level 4 table:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::memory::active_level_4_table;
    use x86_64::VirtAddr;

    println!("Hello World{}", "!");
    blog_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

    for (i, entry) in l4_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 Entry {}: {:?}", i, entry);
        }
    }

    // as before
    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    rusty_os::hlt_loop();
}
```

First, I converted the `physical_memory_offset` of the `BootInfo` struct to a [`VirtAddr`](https://docs.rs/x86_64/0.14.2/x86_64/addr/struct.VirtAddr.html) and pass it to the `active_level_4_table` function. I then used the `iter` function to iterate over the page table entries and the [`enumerate`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.enumerate) combinator to additionally add an index `i` to each element. I only print non-empty entries because all 512 entries wouldn’t fit on the screen.

When I ran it, I saw the following output:
![[Screenshot From 2026-05-21 11-36-37.png]]
I saw that there were various non-empty entries, which all map to different level 3 tables. There were so many regions because kernel code, kernel stack, physical memory mapping, and boot information all used separate memory areas.

To traverse the page tables further and take a look at a level 3 table, I took the mapped frame of an entry and converted it to a virtual address again:

```rust
// in the `for` loop in src/main.rs

use x86_64::structures::paging::PageTable;

if !entry.is_unused() {
    println!("L4 Entry {}: {:?}", i, entry);

    // get the physical address from the entry and convert it
    let phys = entry.frame().unwrap().start_address();
    let virt = phys.as_u64() + boot_info.physical_memory_offset;
    let ptr = VirtAddr::new(virt).as_mut_ptr();
    let l3_table: &PageTable = unsafe { &*ptr };

    // print non-empty entries of the level 3 table
    for (i, entry) in l3_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("  L3 Entry {}: {:?}", i, entry);
        }
    }
}
```

For looking at the level 2 and level 1 tables, I can repeat that process for the level 3 and level 2 entries.

Traversing the page tables manually is interesting because it helps to understand how the CPU performs the translation. However, most of the time, we are only interested in the mapped physical address for a given virtual address, so I created a function for that.

### Translating Addresses

To translate a virtual to a physical address, I had to traverse the four-level page table until I reach the mapped frame. So I created a function that performed this translation:

```rust
// in src/memory.rs

use x86_64::PhysAddr;

/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    translate_addr_inner(addr, physical_memory_offset)
}
```

I forwarded the function to a safe `translate_addr_inner` function to limit the scope of `unsafe`. Rust treats the complete body of an `unsafe fn` like a large unsafe block. By calling into a private safe function, I can make each `unsafe` operation explicit again.

The private inner function contained the real implementation:

```rust
// in src/memory.rs

/// Private function that is called by `translate_addr`.
///
/// This function is safe to limit the scope of `unsafe` because Rust treats
/// the whole body of unsafe functions as an unsafe block. This function must
/// only be reachable through `unsafe fn` from outside of this module.
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // read the active level 4 frame from the CR3 register
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi-level page table
    for &index in &table_indexes {
        // convert the frame into a page table reference
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        // read the page table entry and update `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
```

Instead of reusing the `active_level_4_table` function, I read the level 4 frame from the `CR3` register again. I did this because it simplified this prototype implementation.

The `VirtAddr` struct already provides methods to compute the indexes into the page tables of the four levels. I stored these indexes in a small array because it allowed me to traverse the page tables using a `for` loop. Outside of the loop, I remembered the last visited `frame` to calculate the physical address later. The `frame` points to page table frames while iterating and to the mapped frame after the last iteration, i.e., after following the level 1 entry.

Inside the loop, I again used the `physical_memory_offset` to convert the frame into a page table reference. I then read the entry of the current page table and used the [`PageTableEntry::frame`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page_table/struct.PageTableEntry.html#method.frame) function to retrieve the mapped frame. If the entry is not mapped to a frame, I returned `None`. If the entry maps a huge 2 MiB or 1 GiB page, I panicked.

Then I tested the translation function by translating some addresses:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // new import
    use rusty_os::memory::translate_addr;

    […] // hello world and blog_os::init

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];

    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = unsafe { translate_addr(virt, phys_mem_offset) };
        println!("{:?} -> {:?}", virt, phys);
    }

    […] // test_main(), "it did not crash" printing, and hlt_loop()
}
```

When I ran it, I saw the following output:

![[Screenshot From 2026-05-21 12-03-28.png]]
As expected, the identity-mapped address `0xb8000` translates to the same physical address. The code page and the stack page translate to some arbitrary physical addresses, which depend on how the bootloader created the initial mapping for the kernel. It’s worth noting that the last 12 bits always stay the same after translation, which makes sense because these bits are the [_page offset_](https://os.phil-opp.com/paging-introduction/#paging-on-x86-64) and not part of the translation.

Since each physical address can be accessed by adding the `physical_memory_offset`, the translation of the `physical_memory_offset` address itself should point to physical address `0`. However, the translation fails because the mapping uses huge pages for efficiency, which is not supported in my implementation yet.

### Using `OffsetPageTable`

Translating virtual to physical addresses is a common task in an OS kernel, therefore the `x86_64` crate provided an abstraction for it. The implementation already supports huge pages and several other page table functions apart from `translate_addr`, so I used it in the following instead of adding huge page support to my own implementation.

At the basis of the abstraction are two traits that define various page table mapping functions:

- The [`Mapper`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html) trait is generic over the page size and provides functions that operate on pages. Examples are [`translate_page`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#tymethod.translate_page), which translates a given page to a frame of the same size, and [`map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to), which creates a new mapping in the page table.
- The [`Translate`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Translate.html) trait provides functions that work with multiple page sizes, such as [`translate_addr`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Translate.html#method.translate_addr) or the general [`translate`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Translate.html).

The traits only define the interface, they don’t provide any implementation. The `x86_64` crate provides three types that implement the traits with different requirements in the version I used. The [`OffsetPageTable`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.OffsetPageTable.html) type assumes that the complete physical memory is mapped to the virtual address space at some offset. The [`MappedPageTable`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.MappedPageTable.html) is a bit more flexible: It only requires that each page table frame is mapped to the virtual address space at a calculable address. Finally, the [`RecursivePageTable`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.RecursivePageTable.html) type can be used to access page table frames through [recursive page tables](https://os.phil-opp.com/paging-implementation/#recursive-page-tables).

In my case, the bootloader maps the complete physical memory at a virtual address specified by the `physical_memory_offset` variable, so I used the `OffsetPageTable` type. To initialize it, I created a new `init` function in the `memory` module:

```rust
use x86_64::structures::paging::OffsetPageTable;

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

// make private
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{…}
```

The function takes the `physical_memory_offset` as an argument and returns a new `OffsetPageTable` instance with a `'static` lifetime. This means that the instance stays valid for the complete runtime of the kernel. In the function body, I first called the `active_level_4_table` function to retrieve a mutable reference to the level 4 page table. I then invoked the [`OffsetPageTable::new`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.OffsetPageTable.html#method.new) function with this reference. As the second parameter, the `new` function expects the virtual address at which the mapping of the physical memory starts, which is given in the `physical_memory_offset` variable.

The `active_level_4_table` function should only be called from the `init` function from now on because it can easily lead to aliased mutable references when called multiple times, which can cause undefined behavior. For this reason, I made the function private by removing the `pub` specifier.

I then used the `Translate::translate_addr` method instead of my own `memory::translate_addr` function. I only needed to change a few lines in the `kernel_main`:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // new: different imports
    use rusty_os::memory;
    use x86_64::{structures::paging::Translate, VirtAddr};

    […] // hello world and blog_os::init

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // new: initialize a mapper
    let mapper = unsafe { memory::init(phys_mem_offset) };

    let addresses = […]; // same as before

    for &address in &addresses {
        let virt = VirtAddr::new(address);
        // new: use the `mapper.translate_addr` method
        let phys = mapper.translate_addr(virt);
        println!("{:?} -> {:?}", virt, phys);
    }

    […] // test_main(), "it did not crash" printing, and hlt_loop()
}
```

I needed to imported the `Translate` trait in order to use the [`translate_addr`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Translate.html#method.translate_addr) method it provides.

When I ram it, I saw the same translation results as before, with the difference that the huge page translation now also works:
![[Screenshot From 2026-05-21 16-53-41.png]]
As expected, the translations of `0xb8000` and the code and stack addresses stay the same as with our own translation function. Additionally, we now see that the virtual address `physical_memory_offset` is mapped to the physical address `0x0`.

By using the translation function of the `MappedPageTable` type, I can spare myself the work of implementing huge page support. I also had access to other page functions, such as `map_to`, which I used in the next section.

At this point, we no longer needed the `memory::translate_addr` and `memory::translate_addr_inner` functions.

### Creating a new Mapping

Until now, I only looked at the page tables without modifying anything. So changed that by creating a new mapping for a previously unmapped page.

I used the [`map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to) function of the [`Mapper`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html) trait for my implementation. The documentation told me that it takes four arguments: the page that I want to map, the frame that the page should be mapped to, a set of flags for the page table entry, and a `frame_allocator`. The frame allocator is needed because mapping the given page might require creating additional page tables, which need unused frames as backing storage.

#### A `create_example_mapping` Function

The first step of my implementation was to create a new `create_example_mapping` function that maps a given virtual page to `0xb8000`, the physical frame of the VGA text buffer. I chose that frame because it allowed me to easily test if the mapping was created correctly: I just needed to write to the newly mapped page and see whether I saw the write appeared on the screen.

The `create_example_mapping` function looks like this:

```rust
// in src/memory.rs

use x86_64::{
    PhysAddr,
    structures::paging::{Page, PhysFrame, Mapper, Size4KiB, FrameAllocator}
};

/// Creates an example mapping for the given page to frame `0xb8000`.
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}
```

In addition to the `page` that should be mapped, the function expects a mutable reference to an `OffsetPageTable` instance and a `frame_allocator`. The `frame_allocator` parameter uses the [`impl Trait`](https://doc.rust-lang.org/book/ch10-02-traits.html#traits-as-parameters) syntax to be [generic](https://doc.rust-lang.org/book/ch10-00-generics.html) over all types that implement the [`FrameAllocator`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/trait.FrameAllocator.html) trait. The trait is generic over the [`PageSize`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page/trait.PageSize.html) trait to work with both standard 4 KiB pages and huge 2 MiB/1 GiB pages. I only wanted to create a 4 KiB mapping, so I set the generic parameter to `Size4KiB`.

The [`map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to) method is unsafe because the caller must ensure that the frame is not already in use. The reason for this is that mapping the same frame twice could result in undefined behavior, for example when two different `&mut` references point to the same physical memory location. In my case, I reused the VGA text buffer frame, which is already mapped, so we broke the required condition. However, the `create_example_mapping` function is only a temporary testing function and will be removed later, so it was ok.

In addition to the `page` and the `unused_frame`, the `map_to` method takes a set of flags for the mapping and a reference to the `frame_allocator`. For the flags, I set the `PRESENT` flag because it is required for all valid entries and the `WRITABLE` flag to make the mapped page writable.

The [`map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to) function can fail, so it returns a [`Result`](https://doc.rust-lang.org/core/result/enum.Result.html). Since this was just some example code that does not need to be robust, I just used [`expect`](https://doc.rust-lang.org/core/result/enum.Result.html#method.expect) to panic when an error occurs. On success, the function returns a [`MapperFlush`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.MapperFlush.html) type that provides an easy way to flush the newly mapped page from the translation lookaside buffer (TLB) with its [`flush`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.MapperFlush.html#method.flush) method. Like `Result`, the type uses the [`#[must_use]`](https://doc.rust-lang.org/std/result/#results-must-be-used) attribute to emit a warning when I accidentally forget to use it.

#### A dummy `FrameAllocator`

To be able to call `create_example_mapping`, I needed to create a type that implements the `FrameAllocator` trait first. As noted above, the trait is responsible for allocating frames for new page tables if they are needed by `map_to`.

So I started with the simple case and assumed that I didn’t need to create new page tables. For this case, a frame allocator that always returns `None` sufficed. I created such an `EmptyFrameAllocator` for testing the mapping function:

```rust
// in src/memory.rs

/// A FrameAllocator that always returns `None`.
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}
```

Implementing the `FrameAllocator` is unsafe because the implementer must guarantee that the allocator yields only unused frames. Otherwise, undefined behavior might occur, for example when two virtual pages are mapped to the same physical frame. My `EmptyFrameAllocator` only returns `None`, so this wasn’t a problem in this case.

### Creating the mapping

I now have all the required parameters for calling the `create_example_mapping` function, so I modified the `kernel_main` function to map the page at virtual address `0`. Since I mapped the page to the frame of the VGA text buffer, I should be able to write to the screen through it afterward. The implementation looks like this:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::memory;
    use x86_64::{structures::paging::Page, VirtAddr}; // new import

    […] // hello world and blog_os::init

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = memory::EmptyFrameAllocator;

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    […] // test_main(), "it did not crash" printing, and hlt_loop()
}
```

I first created the mapping for the page at address `0` by calling the `create_example_mapping` function with a mutable reference to the `mapper` and the `frame_allocator` instances. This mapped the page to the VGA text buffer frame, so I should see any write to it on the screen.

Then I converted the page to a raw pointer and wrote a value to offset `400`. I didn’t write to the start of the page because the top line of the VGA buffer is directly shifted off the screen by the next `println`. I wrote the value `0x_f021_f077_f065_f04e`, which represents the string _“New!”_ on a white background. As I learned before, writes to the VGA buffer should be volatile, so I used the [`write_volatile`](https://doc.rust-lang.org/std/primitive.pointer.html#method.write_volatile) method.

When I ran it in QEMU, I saw the following output:
![[Screenshot From 2026-05-21 17-18-10.png]]
The _“New!”_ on the screen was caused by the write to page `0`, which meant that I successfully created a new mapping in the page tables.

Creating that mapping only worked because the level 1 table responsible for the page at address `0` already exists. When I tried to map a page for which no level 1 table existed yet, the `map_to` function failed because it tried to create new page tables by allocating frames with the `EmptyFrameAllocator`. I saw that happened when I tried to map page `0xdeadbeaf000` instead of `0`:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    […]
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    […]
}
```

When we ran it, a panic with the following error message occured:

```
panicked at 'map_to failed: FrameAllocationFailed', /…/result.rs:999:5
```

To map pages that don’t have a level 1 page table yet, I needed to create a proper `FrameAllocator`. But how do I know which frames are unused and how much physical memory is available?

### Allocating Frames

In order to create new page tables, I needed to create a proper frame allocator. To do that, I used the `memory_map` that is passed by the bootloader as part of the `BootInfo` struct:

```rust
// in src/memory.rs

use bootloader::bootinfo::MemoryMap;

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
}
```

The struct has two fields: A `'static` reference to the memory map passed by the bootloader and a `next` field that keeps track of the number of the next frame that the allocator should return.

The memory map is provided by the BIOS/UEFI firmware. It can only be queried very early in the boot process, so the bootloader already calls the respective functions for us. The memory map consists of a list of [`MemoryRegion`](https://docs.rs/bootloader/0.6.4/bootloader/bootinfo/struct.MemoryRegion.html) structs, which contain the start address, the length, and the type (e.g. unused, reserved, etc.) of each memory region.

The `init` function initializes a `BootInfoFrameAllocator` with a given memory map. The `next` field is initialized with `0` and will be increased for every frame allocation to avoid returning the same frame twice. Since I didn’t know if the usable frames of the memory map were already used somewhere else, our `init` function must be `unsafe` to require additional guarantees from the caller.

#### A `usable_frames` Method

Before I implement the `FrameAllocator` trait, I added an auxiliary method that converted the memory map into an iterator of usable frames:

```rust
// in src/memory.rs

use bootloader::bootinfo::MemoryRegionType;

impl BootInfoFrameAllocator {
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}
```

This function used iterator combinator methods to transform the initial `MemoryMap` into an iterator of usable physical frames:

- First, I called the `iter` method to convert the memory map to an iterator of [`MemoryRegion`](https://docs.rs/bootloader/0.6.4/bootloader/bootinfo/struct.MemoryRegion.html)s.
- Then I used the [`filter`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.filter) method to skip any reserved or otherwise unavailable regions. The bootloader updated the memory map for all the mappings it creates, so frames that are used by the kernel (code, data, or stack) or to store the boot information are already marked as `InUse` or similar. Thus, I can be sure that `Usable` frames are not used somewhere else.
- Afterwards, I used the [`map`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.map) combinator and Rust’s [range syntax](https://doc.rust-lang.org/core/ops/struct.Range.html) to transform our iterator of memory regions to an iterator of address ranges.
- Next, I used [`flat_map`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.flat_map) to transform the address ranges into an iterator of frame start addresses, choosing every 4096th address using [`step_by`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.step_by). Since 4096 bytes (= 4 KiB) is the page size, I got the start address of each frame. The bootloader page-aligns all usable memory areas so that we don’t need any alignment or rounding code here. By using [`flat_map`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.flat_map) instead of `map`, I got an `Iterator<Item = u64>` instead of an `Iterator<Item = Iterator<Item = u64>>`.
- Finally, I converted the start addresses to `PhysFrame` types to construct an `Iterator<Item = PhysFrame>`.

The return type of the function uses the [`impl Trait`](https://doc.rust-lang.org/book/ch10-02-traits.html#returning-types-that-implement-traits) feature. This way, I can specify that we return some type that implements the [`Iterator`](https://doc.rust-lang.org/core/iter/trait.Iterator.html) trait with item type `PhysFrame` but don’t need to name the concrete return type. This was important here because I _can’t_ name the concrete type since it depends on unnamable closure types.

#### Implementing the `FrameAllocator` Trait

Then I implemented the `FrameAllocator` trait:

```rust
// in src/memory.rs

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
```

I first used the `usable_frames` method to get an iterator of usable frames from the memory map. Then, I used the [`Iterator::nth`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.nth) function to get the frame with index `self.next` (thereby skipping `(self.next - 1)` frames). Before returning that frame, I increased `self.next` by one so that I returned the following frame on the next call.

This implementation is not quite optimal since it recreated the `usable_frame` allocator on every allocation. It would be better to directly store the iterator as a struct field instead. Then we wouldn’t need the `nth` method and could just call [`next`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#tymethod.next) on every allocation. The problem with this approach was that it was not possible to store an `impl Trait` type in a struct field currently. It might work someday when [_named existential types_](https://github.com/rust-lang/rfcs/pull/2071) are fully implemented.

#### Using the `BootInfoFrameAllocator`

I can now modify our `kernel_main` function to pass a `BootInfoFrameAllocator` instance instead of an `EmptyFrameAllocator`:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::memory::BootInfoFrameAllocator;
    […]
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    […]
}
```

With the boot info frame allocator, the mapping succeeds and I saw the black-on-white _“New!”_ on the screen again. Behind the scenes, the `map_to` method creates the missing page tables in the following way:

- Use the passed `frame_allocator` to allocate an unused frame.
- Zero the frame to create a new, empty page table.
- Map the entry of the higher level table to that frame.
- Continue with the next table level.

While the `create_example_mapping` function is just some example code, I am now able to create new mappings for arbitrary pages. This will be essential for allocating memory or implementing multithreading in future section.

---
