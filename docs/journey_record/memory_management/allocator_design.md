(May 23, 2026)

---

This is the Day 17 in my `"Writing my own OS from scratch in Rust` journey - `rusty_os`.

First I learned the memory concept and Rust allocation methods which is explained [here](topics/Memory_management).

Then I created a minimal implementation of Rust’s allocator interface using a dummy allocator, which helped me create a proper heap memory region for the kernel. For that, I defined a virtual address range for the heap and then mapped all pages of that range to physical frames using the `Mapper` and `FrameAllocator` from the previous implementation.

Finally, I added a dependency on the `linked_list_allocator` crate to add a proper allocator to the kernel. With this allocator, I was able to use `Box`, `Vec`, and other allocation and collection types from the `alloc` crate.

---

### The Allocator Interface

The first step in implementing a heap allocator was to add a dependency on the built-in [`alloc`](https://doc.rust-lang.org/alloc/) crate. Like the [`core`](https://doc.rust-lang.org/core/) crate, it is a subset of the standard library that additionally contains the allocation and collection types. To add the dependency on `alloc`, I added the following to the `lib.rs`:

```rust
// in src/lib.rs

extern crate alloc;
```

Contrary to normal dependencies, I don’t need to modify the `Cargo.toml`. The reason is that the `alloc` crate ships with the Rust compiler as part of the standard library, so the compiler already knows about the crate. By adding this `extern crate` statement, I specified that the compiler should try to include it. (Historically, all dependencies needed an `extern crate` statement, which is now optional).

Since I am compiling for a custom target, I can’t use the precompiled version of `alloc` that is shipped with the Rust installation. Instead, I have to tell cargo to recompile the crate from source. I did that by adding it to the `unstable.build-std` array in the `.cargo/config.toml` file:

```toml
# in .cargo/config.toml

[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
```

Now the compiler will recompile and include the `alloc` crate in the kernel.

The reason that the `alloc` crate is disabled by default in `#[no_std]` crates is that it has additional requirements. When I tried to compile my project, I saw these requirements as errors:

```
error: no global memory allocator found but one is required; link to std or add
       #[global_allocator] to a static item that implements the GlobalAlloc trait.
```

The error occurred because the `alloc` crate requires a heap allocator, which is an object that provides the `allocate` and `deallocate` functions. In Rust, heap allocators are described by the [`GlobalAlloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) trait, which was mentioned in the error message. To set the heap allocator for the crate, the `#[global_allocator]` attribute must be applied to a `static` variable that implements the `GlobalAlloc` trait.

---

### The GlobalAlloc Trait

The [`GlobalAlloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) trait defines the functions that a heap allocator must provide. The trait is special because it is almost never used directly by the programmer. Instead, the compiler will automatically insert the appropriate calls to the trait methods when using the allocation and collection types of `alloc`.

As I learned Rust allocation methods, I saw this `GlobalAlloc` trait:

```rust
pub unsafe trait GlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { ... }
    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: Layout,
        new_size: usize
    ) -> *mut u8 { ... }
}
```

It defines the two required methods [`alloc`](https://doc.rust-lang.org/alloc/) and [`dealloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.dealloc), which correspond to the `allocate` and `deallocate` functions I used in the examples:

- The [`alloc`](https://doc.rust-lang.org/alloc/) method takes a [`Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html) instance as an argument, which describes the desired size and alignment that the allocated memory should have. It returns a [raw pointer](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html#dereferencing-a-raw-pointer) to the first byte of the allocated memory block. Instead of an explicit error value, the `alloc` method returns a null pointer to signal an allocation error. This is a bit non-idiomatic, but it has the advantage that wrapping existing system allocators is easy since they use the same convention.
- The [`dealloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.dealloc) method is the counterpart and is responsible for freeing a memory block again. It receives two arguments: the pointer returned by `alloc` and the `Layout` that was used for the allocation.

The trait additionally defines the two methods [`alloc_zeroed`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#method.alloc_zeroed) and [`realloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#method.realloc) with default implementations:

- The [`alloc_zeroed`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#method.alloc_zeroed) method is equivalent to calling `alloc` and then setting the allocated memory block to zero, which is exactly what the provided default implementation does. An allocator implementation can override the default implementations with a more efficient custom implementation if possible.
- The [`realloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#method.realloc) method allows to grow or shrink an allocation. The default implementation allocates a new memory block with the desired size and copies over all the content from the previous allocation. Again, an allocator implementation can probably provide a more efficient implementation of this method, for example by growing/shrinking the allocation in-place if possible.

#### Unsafety

One thing to notice is that both the trait itself and all trait methods are declared as `unsafe`:

- The reason for declaring the trait as `unsafe` is that the programmer must guarantee that the trait implementation for an allocator type is correct. For example, the `alloc` method must never return a memory block that is already used somewhere else because this would cause undefined behavior.
- Similarly, the reason that the methods are `unsafe` is that the caller must ensure various invariants when calling the methods, for example, that the `Layout` passed to `alloc` specifies a non-zero size. This is not really relevant in practice since the methods are normally called directly by the compiler, which ensures that the requirements are met.

### A `DummyAllocator`

Now that I knew what an allocator type should provide, I can create a simple dummy allocator. For that, I created a new `allocator` module:

```rust
// in src/lib.rs

pub mod allocator;
```

The dummy allocator does the absolute minimum to implement the trait and always returns an error when `alloc` is called. It looked like this:

```rust
// in src/allocator.rs

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}
```

The struct does not need any fields, so I create it as a [zero-sized type](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts). As mentioned above, I always returned the null pointer from `alloc`, which corresponds to an allocation error. Since the allocator never returns any memory, a call to `dealloc` should never occur. For this reason, I simply panicked in the `dealloc` method. The `alloc_zeroed` and `realloc` methods have default implementations, so I didn’t need to provide implementations for them.

I now have a simple allocator, but I still had to tell the Rust compiler that it should use this allocator. This was where the `#[global_allocator]` attribute came in.

### The `#[global_allocator]` Attribute

The `#[global_allocator]` attribute tells the Rust compiler which allocator instance it should use as the global heap allocator. The attribute is only applicable to a `static` that implements the `GlobalAlloc` trait. So I registered an instance of the `Dummy` allocator as the global allocator:

```rust
// in src/allocator.rs

#[global_allocator]
static ALLOCATOR: Dummy = Dummy;
```

Since the `Dummy` allocator is a [zero-sized type](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts), I didn’t need to specify any fields in the initialization expression.

With this static, the compilation errors should be fixed. Now I can use the allocation and collection types of `alloc`. For example, I can use a [`Box`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html) to allocate a value on the heap:

```rust
// in src/main.rs

extern crate alloc;

use alloc::boxed::Box;

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // […] print "Hello World!", call `init`, create `mapper` and `frame_allocator`

    let x = Box::new(41);

    // […] call `test_main` in test mode

    println!("It did not crash!");
    blog_os::hlt_loop();
}

```

When I ran the above code, I saw that a panic occurred:

![[Screenshot From 2026-05-23 18-00-12.png]]
The panic occurred because the `Box::new` function implicitly called the `alloc` function of the global allocator. The dummy allocator always returned a null pointer, so every allocation failed. To fix this, I needed to create an allocator that actually returns usable memory.

---

### Creating a Kernel Heap

Before I can create a proper allocator, I first needed to create a heap memory region from which the allocator can allocate memory. To do this, I needed to define a virtual memory range for the heap region and then map this region to physical frames.

The first step was to define a virtual memory region for the heap. I can choose any virtual address range that I like, as long as it is not already used for a different memory region. I defined it as the memory starting at address `0x_4444_4444_0000` so that I can easily recognize a heap pointer later:

```rust
// in src/allocator.rs

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
```

I set the heap size to 100 KiB for now. If I need more space in the future, I can simply increase it.

If I tried to use this heap region now, a page fault would occur since the virtual memory region is not mapped to physical memory yet. To resolve this, I created an `init_heap` function that maps the heap pages using the [`Mapper` API](https://os.phil-opp.com/paging-implementation/#using-offsetpagetable):

```rust
// in src/allocator.rs

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    Ok(())
}
```

The function takes mutable references to a [`Mapper`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html) and a [`FrameAllocator`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/trait.FrameAllocator.html) instance, both limited to 4 KiB pages by using [`Size4KiB`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page/enum.Size4KiB.html) as the generic parameter. The return value of the function is a [`Result`](https://doc.rust-lang.org/core/result/enum.Result.html) with the unit type `()` as the success variant and a [`MapToError`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/enum.MapToError.html) as the error variant, which is the error type returned by the [`Mapper::map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to) method. Reusing the error type makes sense here because the `map_to` method is the main source of errors in this function.

The implementation can be broken down into two parts:

- **Creating the page range:**: To create a range of the pages that I want to map, I converted the `HEAP_START` pointer to a [`VirtAddr`](https://docs.rs/x86_64/0.14.2/x86_64/addr/struct.VirtAddr.html) type. Then I calculate the heap end address from it by adding the `HEAP_SIZE`. I want an inclusive bound (the address of the last byte of the heap), so I subtract 1. Next, I convert the addresses into [`Page`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page/struct.Page.html) types using the [`containing_address`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page/struct.Page.html#method.containing_address) function. Finally, I create a page range from the start and end pages using the [`Page::range_inclusive`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/page/struct.Page.html#method.range_inclusive) function.
    
- **Mapping the pages:** The second step is to map all pages of the page range I just created. For that, I iterate over these pages using a `for` loop. For each page, I do the following:
    
    - I allocate a physical frame that the page should be mapped to using the [`FrameAllocator::allocate_frame`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/trait.FrameAllocator.html#tymethod.allocate_frame) method. This method returns [`None`](https://doc.rust-lang.org/core/option/enum.Option.html#variant.None) when there are no more frames left. I deal with that case by mapping it to a [`MapToError::FrameAllocationFailed`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/enum.MapToError.html#variant.FrameAllocationFailed) error through the [`Option::ok_or`](https://doc.rust-lang.org/core/option/enum.Option.html#method.ok_or) method and then applying the [question mark operator](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html) to return early in the case of an error.
        
    - I set the required `PRESENT` flag and the `WRITABLE` flag for the page. With these flags, both read and write accesses are allowed, which makes sense for heap memory.
        
    - I use the [`Mapper::map_to`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/trait.Mapper.html#method.map_to) method for creating the mapping in the active page table. The method can fail, so I use the [question mark operator](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html) again to forward the error to the caller. On success, the method returns a [`MapperFlush`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.MapperFlush.html) instance that I can use to update the [_translation lookaside buffer_](https://os.phil-opp.com/paging-introduction/#the-translation-lookaside-buffer) using the [`flush`](https://docs.rs/x86_64/0.14.2/x86_64/structures/paging/mapper/struct.MapperFlush.html#method.flush) method.
        

The final step was to call this function from the `kernel_main`:

```rust
// in src/main.rs

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::allocator; // new import
    use rusty_os::memory::{self, BootInfoFrameAllocator};

    println!("Hello World{}", "!");
    rusty_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // new
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let x = Box::new(41);

    // […] call `test_main` in test mode

    println!("It did not crash!");
    rusty_os::hlt_loop();
}
```

In case the `init_heap` function returns an error, I panicked using the [`Result::expect`](https://doc.rust-lang.org/core/result/enum.Result.html#method.expect) method since there was currently no sensible way for me to handle this error.

I now had a mapped heap memory region that was ready to be used. The `Box::new` call still used the old `Dummy` allocator, so I still saw the “out of memory” error when I ran it. So I fixed this by using a proper allocator.

---

### Using an Allocator Crate

Since implementing an allocator is somewhat complex, I start by using an external allocator crate. I will learn how to implement my own allocator in future.

A simple allocator crate for `no_std` applications is the [`linked_list_allocator`](https://github.com/phil-opp/linked-list-allocator/) crate. Its name comes from the fact that it uses a linked list data structure to keep track of deallocated memory regions.

I added a dependency in our `Cargo.toml`:

```toml
# in Cargo.toml

[dependencies]
linked_list_allocator = "0.9.0"
```

Then I replaced the dummy allocator with the allocator provided by the crate:

```rust
// in src/allocator.rs

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
```

The struct is named `LockedHeap` because it uses the [`spinning_top::Spinlock`](https://docs.rs/spinning_top/0.1.0/spinning_top/type.Spinlock.html) type for synchronization. This was required because multiple threads could access the `ALLOCATOR` static at the same time. As always, when using a spinlock or a mutex, I need to be careful to not accidentally cause a deadlock. This means that I shouldn’t perform any allocations in interrupt handlers, since they can run at an arbitrary time and might interrupt an in-progress allocation.

Setting the `LockedHeap` as global allocator is not enough. The reason was that I use the [`empty`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.LockedHeap.html#method.empty) constructor function, which creates an allocator without any backing memory. Like the dummy allocator, it always returns an error on `alloc`. To fix this, I needed to initialize the allocator after creating the heap:

```rust
// in src/allocator.rs

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // […] map all heap pages to physical frames

    // new
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
```

I used the [`lock`](https://docs.rs/lock_api/0.3.3/lock_api/struct.Mutex.html#method.lock) method on the inner spinlock of the `LockedHeap` type to get an exclusive reference to the wrapped [`Heap`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html) instance, on which I then called the [`init`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html#method.init) method with the heap bounds as arguments. Because the [`init`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html#method.init) function already tries to write to the heap memory, I must initialize the heap only _after_ mapping the heap pages.

After initializing the heap, I can now use all allocation and collection types of the built-in [`alloc`](https://doc.rust-lang.org/alloc/) crate without error:

```rust
// in src/main.rs

use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // […] initialize interrupts, mapper, frame_allocator, heap

    // allocate a number on the heap
    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    // […] call `test_main` in test context
    println!("It did not crash!");
    rusty_os::hlt_loop();
}
```

This code example shows some uses of the [`Box`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html), [`Vec`](https://doc.rust-lang.org/alloc/vec/), and [`Rc`](https://doc.rust-lang.org/alloc/rc/) types. For the `Box` and `Vec` types, I printed the underlying heap pointers using the [`{:p}` formatting specifier](https://doc.rust-lang.org/core/fmt/trait.Pointer.html). To showcase `Rc`, I created a reference-counted heap value and used the [`Rc::strong_count`](https://doc.rust-lang.org/alloc/rc/struct.Rc.html#method.strong_count) function to print the current reference count before and after dropping an instance (using [`core::mem::drop`](https://doc.rust-lang.org/core/mem/fn.drop.html)).

When I ran it, we saw the following:
![[Screenshot From 2026-05-23 18-43-53.png]]
As expected, I saw that the `Box` and `Vec` values lived on the heap, as indicated by the pointer starting with the `0x_4444_4444_*` prefix. The reference counted value also behaved as expected, with the reference count being 2 after the `clone` call, and 1 again after one of the instances was dropped.

The reason that the vector started at offset `0x800` is not that the boxed value is `0x800` bytes large, but the [reallocations](https://doc.rust-lang.org/alloc/vec/struct.Vec.html#capacity-and-reallocation) that occurred when the vector needed to increase its capacity. For example, when the vector’s capacity is 32 and we try to add the next element, the vector allocates a new backing array with a capacity of 64 behind the scenes and copies all elements over. Then it frees the old allocation.

---

### Adding a Test

To ensure that I don’t accidentally break my new allocation code, I should add an integration test for it. I started by creating a new `tests/heap_allocation.rs` file with the following content:

```rust
// in tests/heap_allocation.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    unimplemented!();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}
```

I reuseed the `test_runner` and `test_panic_handler` functions from the `lib.rs`. Since I wanted to test allocations, I enabled the `alloc` crate through the `extern crate alloc` statement.

The implementation of the `main` function looks like this:

```rust
// in tests/heap_allocation.rs

fn main(boot_info: &'static BootInfo) -> ! {
    use rusty_os::allocator;
    use rusty_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    blog_os::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    test_main();
    loop {}
}
```

It is very similar to the `kernel_main` function in the `main.rs`, with the differences that I didn’t invoke `println`, don’t include any example allocations, and called `test_main` unconditionally.

Then I first added a test that performs some simple allocations using [`Box`](https://doc.rust-lang.org/alloc/boxed/struct.Box.html) and checked the allocated values to ensure that basic allocations work:

```rust
// in tests/heap_allocation.rs
use alloc::boxed::Box;

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}
```

Most importantly, this test verified that no allocation error occurred.

Next, I iteratively build a large vector, to test both large allocations and multiple allocations (due to reallocations):

```rust
// in tests/heap_allocation.rs

use alloc::vec::Vec;

#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}
```

I verified the sum by comparing it with the formula for the [n-th partial sum](https://en.wikipedia.org/wiki/1_%2B_2_%2B_3_%2B_4_%2B_%E2%8B%AF#Partial_sums). This gave me some confidence that the allocated values are all correct.

As a third test, I created ten thousand allocations after each other:

```rust
// in tests/heap_allocation.rs

use blog_os::allocator::HEAP_SIZE;

#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}
```

This test ensures that the allocator reuses freed memory for subsequent allocations since it would run out of memory otherwise. This might seem like an obvious requirement for an allocator, but there are allocator designs that don’t do this. An example is the bump allocator design.

Finally I ran new integration test:

```
> cargo test --test heap_allocation
[…]
Running 3 tests
simple_allocation... [ok]
large_vec... [ok]
many_boxes... [ok]
```

All three tests succeeded! 

---
