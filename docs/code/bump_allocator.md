I started the implementation by declaring a new `allocator::bump` submodule:

```rust
// in src/allocator.rs

pub mod bump;
```

The content of the submodule lives in a new `src/allocator/bump.rs` file, which I created with the following content:

```rust
// in src/allocator/bump.rs

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty bump allocator.
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// This method is unsafe because the caller must ensure that the given
    /// memory range is unused. Also, this method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}
```

### Field Explanations

|Field|Purpose|Why It Exists|
|---|---|---|
|`heap_start`|Lower bound of heap memory|Remember where the heap begins, needed for resetting|
|`heap_end`|Upper bound of heap memory|Perform bounds checking (prevent allocating past heap)|
|`next`|Pointer to next free byte|Core of bump allocator - moves forward on each allocation|
|`allocations`|Counter of active allocations|Know when to reset (when count reaches zero)|

### Visual Representation

```text

Memory layout after initialization:
heap_start = 0x1000
heap_end   = 0x3000 (8KB heap)
next       = 0x1000 (points to start)
[0x1000]─────────────────────────────────────[0x3000]
  ↑                                      ↑
next                              heap_end (unused)
After 3 allocations (16, 32, 64 bytes):
[16B][32B][64B]←─────── Free space ───────→
 ↑    ↑    ↑         ↑
alloc1 alloc2 alloc3 next = 0x1000 + 16+32+64 = 0x10D0
```

## Constructor and Initialization

### The `new()` Function

```rust

pub const fn new() -> Self {
    BumpAllocator {
        heap_start: 0,
        heap_end: 0,
        next: 0,
        allocations: 0,
    }
}
```

**Why `const fn`?**

- Global statics must be initialized at compile time
    
- Regular functions can't run at compile time
    
- `const fn` guarantees compile-time evaluation
    

**Why all zeros?**

- The allocator starts in an "uninitialized" state
    
- Actual heap bounds are unknown at compile time (depends on linker script, memory layout)
    
- We'll set real values later with `init()`
    

### The `init()` Function

```rust

pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
    self.heap_start = heap_start;
    self.heap_end = heap_start + heap_size;
    self.next = heap_start;
}
```

**Why `unsafe`?**

- The caller must guarantee that `heap_start` to `heap_end` is valid memory
    
- This memory range must not be used by anything else
    
- The caller must ensure this is called only once (double initialization would corrupt memory)
    

**Why separate `init` from `new`?**

- Keeps interface identical to other allocators (like linked_list_allocator)
    
- Allows switching allocators without code changes
    
- Separates creation from configuration (good design)

The `heap_start` and `heap_end` fields keep track of the lower and upper bounds of the heap memory region. The caller needs to ensure that these addresses are valid, otherwise the allocator would return invalid memory. For this reason, the `init` function needs to be `unsafe` to call.

The purpose of the `next` field is to always point to the first unused byte of the heap, i.e., the start address of the next allocation. It is set to `heap_start` in the `init` function because at the beginning, the entire heap is unused. On each allocation, this field will be increased by the allocation size (_“bumped”_) to ensure that I don’t return the same memory region twice.

The `allocations` field is a simple counter for the active allocations with the goal of resetting the allocator after the last allocation has been freed. It is initialized with 0.

I chose to create a separate `init` function instead of performing the initialization directly in `new` in order to keep the interface identical to the allocator provided by the `linked_list_allocator` crate. This way, the allocators can be switched without additional code changes.

### Implementing `GlobalAlloc`

All heap allocators need to implement the [`GlobalAlloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) trait, which is defined like this:

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

Only the `alloc` and `dealloc` methods are required; the other two methods have default implementations and can be omitted.

#### First Implementation Attempt

I tried to implement the `alloc` method for the `BumpAllocator`:

```rust
// in src/allocator/bump.rs

use alloc::alloc::{GlobalAlloc, Layout};

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // TODO alignment and bounds check
        let alloc_start = self.next;
        self.next = alloc_start + layout.size();
        self.allocations += 1;
        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        todo!();
    }
}
```

## The Mutability Problem

### The Core Issue

```rust

// GlobalAlloc trait requires &self (immutable reference)
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.next = ???  // ERROR: can't modify &self!
    }
}
```


**Why does GlobalAlloc use `&self`?**

- Global allocator is stored in a `static` variable
    
- Statics in Rust are immutable by default
    
- You cannot call `&mut self` methods on a static
    

**The contradiction:**

- Bump allocator MUST modify `next` on every allocation
    
- GlobalAlloc only gives us `&self` references
    
- We need a way to get interior mutability

First, I used the `next` field as the start address for the allocation. Then I updated the `next` field to point to the end address of the allocation, which is the next unused address on the heap. Before returning the start address of the allocation as a `*mut u8` pointer, I increased the `allocations` counter by 1.

Note that I didn’t perform any bounds checks or alignment adjustments, so this implementation is not safe yet. This did not matter much because it failed to compile anyway with the following error:

```
error[E0594]: cannot assign to `self.next` which is behind a `&` reference
  --> src/allocator/bump.rs:29:9
   |
29 |         self.next = alloc_start + layout.size();
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `self` is a `&` reference, so the data it refers to cannot be written
```

(The same error also occurs for the `self.allocations += 1` line. I omitted it here for brevity.)

The error occurred because the [`alloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.alloc) and [`dealloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html#tymethod.dealloc) methods of the `GlobalAlloc` trait only operate on an immutable `&self` reference, so updating the `next` and `allocations` fields is not possible. This is problematic because updating `next` on every allocation is the essential principle of a bump allocator.

---
#### `GlobalAlloc` and Mutability

Before I looked at a possible solution to this mutability problem, I tried to understand why the `GlobalAlloc` trait methods are defined with `&self` arguments: the global heap allocator is defined by adding the `#[global_allocator]` attribute to a `static` that implements the `GlobalAlloc` trait. Static variables are immutable in Rust, so there is no way to call a method that takes `&mut self` on the static allocator. For this reason, all the methods of `GlobalAlloc` only take an immutable `&self` reference.

Fortunately, there was a way to get a `&mut self` reference from a `&self` reference: I can use synchronized [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html) by wrapping the allocator in a [`spin::Mutex`](https://docs.rs/spin/0.5.0/spin/struct.Mutex.html) spinlock. This type provides a `lock` method that performs [mutual exclusion](https://en.wikipedia.org/wiki/Mutual_exclusion) and thus safely turns a `&self` reference to a `&mut self` reference. I’ve already used the wrapper type multiple times in the kernel.

#### A `Locked` Wrapper Type

With the help of the `spin::Mutex` wrapper type, I can implement the `GlobalAlloc` trait for the bump allocator. The trick is to implement the trait not for the `BumpAllocator` directly, but for the wrapped `spin::Mutex<BumpAllocator>` type:

```rust
unsafe impl GlobalAlloc for spin::Mutex<BumpAllocator> {…}
```

Unfortunately, this still didn’t work because the Rust compiler does not permit trait implementations for types defined in other crates:

```
error[E0117]: only traits defined in the current crate can be implemented for arbitrary types
  --> src/allocator/bump.rs:28:1
   |
28 | unsafe impl GlobalAlloc for spin::Mutex<BumpAllocator> {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^--------------------------
   | |                           |
   | |                           `spin::mutex::Mutex` is not defined in the current crate
   | impl doesn't use only types from inside the current crate
   |
   = note: define and implement a trait or new type instead
```

To fix this, I needed to create my own wrapper type around `spin::Mutex`:

```rust
// in src/allocator.rs

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
```

The type is a generic wrapper around a `spin::Mutex<A>`. It imposes no restrictions on the wrapped type `A`, so it can be used to wrap all kinds of types, not just allocators. It provides a simple `new` constructor function that wraps a given value. For convenience, it also provides a `lock` function that calls `lock` on the wrapped `Mutex`. Since the `Locked` type is general enough to be useful for other allocator implementations too, I put it in the parent `allocator` module.

### Why a Wrapper Instead of Using Mutex Directly?

```rust

// This doesn't work:
unsafe impl GlobalAlloc for spin::Mutex<BumpAllocator> {
    // ERROR: can't implement trait for type from another crate
}
```

**The Orphan Rule:**  
Rust prevents implementing a foreign trait on a foreign type. Both `GlobalAlloc` (from `alloc` crate) and `spin::Mutex` (from `spin` crate) are foreign.

### Our Locked Wrapper

```rust

pub struct Locked<A> {
    inner: spin::Mutex<A>,
}
impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
```

**Why this works:**

- `Locked` is defined in our crate (not foreign)
    
- We can implement `GlobalAlloc` for `Locked<BumpAllocator>`
    
- The wrapper provides interior mutability through `spin::Mutex`
    

**What `spin::Mutex` does:**

- Provides safe interior mutability even with `&self`
    
- `lock()` returns a `MutexGuard` that implements `DerefMut`
    
- The guard gives us `&mut BumpAllocator` access
    
- Only one thread can hold the lock at a time (prevents data races)

#### Implementation for `Locked<BumpAllocator>`

The `Locked` type is defined in my own crate (in contrast to `spin::Mutex`), so I can use it to implement `GlobalAlloc` for the bump allocator. The full implementation looks like this:

```rust
// in src/allocator/bump.rs

use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock(); // get a mutable reference

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // out of memory
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock(); // get a mutable reference

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
```

The first step for both `alloc` and `dealloc` was to call the [`Mutex::lock`](https://docs.rs/spin/0.5.0/spin/struct.Mutex.html#method.lock) method through the `inner` field to get a mutable reference to the wrapped allocator type. The instance remains locked until the end of the method, so that no data race can occur in multithreaded contexts.

Compared to the previous prototype, the `alloc` implementation now respects alignment requirements and performs a bounds check to ensure that the allocations stay inside the heap memory region. The first step is to round up the `next` address to the alignment specified by the `Layout` argument. I then add the requested allocation size to `alloc_start` to get the end address of the allocation. To prevent integer overflow on large allocations, I use the [`checked_add`](https://doc.rust-lang.org/std/primitive.usize.html#method.checked_add) method. If an overflow occurs or if the resulting end address of the allocation is larger than the end address of the heap, I return a null pointer to signal an out-of-memory situation. Otherwise, I update the `next` address and increase the `allocations` counter by 1 like before. Finally, I return the `alloc_start` address converted to a `*mut u8` pointer.

The `dealloc` function ignores the given pointer and `Layout` arguments. Instead, it just decreases the `allocations` counter. If the counter reaches `0` again, it means that all allocations were freed again. In this case, it resets the `next` address to the `heap_start` address to make the complete heap memory available again.

### Address Alignment

The `align_up` function is general enough that I can put it into the parent `allocator` module. A basic implementation looks like this:

```rust
// in src/allocator.rs

/// Align the given address `addr` upwards to alignment `align`.
fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr // addr already aligned
    } else {
        addr - remainder + align
    }
}
```

The function first computes the [remainder](https://en.wikipedia.org/wiki/Euclidean_division) of the division of `addr` by `align`. If the remainder is `0`, the address is already aligned with the given alignment. Otherwise, I align the address by subtracting the remainder (so that the new remainder is 0) and then adding the alignment (so that the address does not become smaller than the original address).

Note that this isn’t the most efficient way to implement this function. A much faster implementation looks like this:

```rust
/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a poIr of two.
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
```

This method requires `align` to be a poIr of two, which can be guaranteed by utilizing the `GlobalAlloc` trait (and its [`Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html) parameter). This makes it possible to create a [bitmask](https://en.wikipedia.org/wiki/Mask_\(computing\)) to align the address in a very efficient way. To understand how it works, let’s go through it step by step, starting on the right side:

- Since `align` is a poIr of two, its [binary representation](https://en.wikipedia.org/wiki/Binary_number#Representation) has only a single bit set (e.g. `0b000100000`). This means that `align - 1` has all the loIr bits set (e.g. `0b00011111`).
- By creating the [bitwise `NOT`](https://en.wikipedia.org/wiki/Bitwise_operation#NOT) through the `!` operator, I get a number that has all the bits set except for the bits loIr than `align` (e.g. `0b…111111111100000`).
- By performing a [bitwise `AND`](https://en.wikipedia.org/wiki/Bitwise_operation#AND) on an address and `!(align - 1)`, I align the address _downwards_. This works by clearing all the bits that are loIr than `align`.
- Since I want to align upwards instead of downwards, I increase the `addr` by `align - 1` before performing the bitwise `AND`. This way, already aligned addresses remain the same while non-aligned addresses are rounded to the next alignment boundary.

Both compute the same result, only using different methods.

### Step-by-Step Logic

#### Step 1: Acquire the Lock

```rust

let mut bump = self.lock();

```

- Locks the mutex (spins if already locked)
    
- Returns a guard that dereferences to `&mut BumpAllocator`
    
- Lock held until `bump` goes out of scope (end of function)
    

#### Step 2: Align the Address

```rust

let alloc_start = align_up(bump.next, layout.align());
```

**Why alignment matters:**

- CPU requires certain types to be at specific addresses
    
- Example: `u64` must be 8-byte aligned (address divisible by 8)
    
- Without alignment: CPU fault or performance penalty
    

**Example of alignment:**

```text

bump.next = 0x1001 (not aligned to 8)
layout.align() = 8
align_up(0x1001, 8) = 0x1008
bump.next = 0x1008 (already aligned)
align_up(0x1008, 8) = 0x1008
```

#### Step 3: Calculate End Address with Overflow Check

```rust

let alloc_end = match alloc_start.checked_add(layout.size()) {
    Some(end) => end,
    None => return ptr::null_mut(),
};
```

**Why `checked_add`?**

- Prevents integer overflow (security vulnerability)
    
- Example: `alloc_start = usize::MAX`, `size = 1` would overflow to 0
    
- Overflow could bypass bounds checks (alloc_end becomes less than heap_start)
    

#### Step 4: Bounds Check

```rust

if alloc_end > bump.heap_end {
    ptr::null_mut() // Out of memory
}
```

**What we're checking:**

- Does the allocation fit within the heap?
    
- `alloc_end` must be ≤ `heap_end`
    
- Returns null pointer (standard OOM signal)
    

#### Step 5: Update State and Return

```rust

bump.next = alloc_end;        // Move next pointer forward
bump.allocations += 1;        // Track active allocations
alloc_start as *mut u8        // Return pointer to user
```

---
##  The Dealloc Implementation

```rust

unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
    let mut bump = self.lock();
    
    bump.allocations -= 1;
    if bump.allocations == 0 {
        bump.next = bump.heap_start;
    }
}
```

### Key Design Decisions

#### Why Ignore `_ptr` and `_layout`?

- Bump allocator doesn't track individual allocations
    
- Cannot free memory in the middle of the heap
    
- Only knows total count of active allocations
    
#### The Reset Strategy

```text

Scenario: 3 allocations active
- alloc1, alloc2, alloc3 allocated
- allocations = 3
Deallocations:
- free(alloc3) → allocations = 2
- free(alloc2) → allocations = 1
- free(alloc1) → allocations = 0 → RESET!
After reset:
next = heap_start (entire heap available again)
```

**Why reset only when count reaches zero?**

- All allocations were freed in LIFO order (last allocated, first freed)
    
- Bump allocator can't free middle allocations
    
- Resetting earlier would cause memory corruption
    

#### The Limitation

This dealloc strategy only works if allocations are freed in reverse order:

```text

Good: alloc A, B, C → free C, B, A
Bad:  alloc A, B, C → free A (can't reset)
```

---
## The Alignment Function

### Efficient Power-of-Two Alignment

```rust

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
```

**How it works with example:**

```text

addr = 0x1001, align = 8
Step 1: addr + align - 1
0x1001 + 7 = 0x1008
Step 2: !(align - 1)  [bitwise NOT]
align - 1 = 7 = 0b0111
!(7) = ...11111000  (all bits except lower 3)
Step 3: Bitwise AND
0x1008 & (...11111000) = 0x1008 ✓
Another example (already aligned):
addr = 0x1008, align = 8
0x1008 + 7 = 0x100F
0x100F & ...11111000 = 0x1008 ✓
```

**Why power of two?**

- Alignments in computers are always powers of two (1, 2, 4, 8, 16...)
    
- `Layout::align()` guarantees power of two
    
- Enables this efficient bit manipulation

---
### Using It

To use the bump allocator instead of the `linked_list_allocator` crate, I needed to update the `ALLOCATOR` static in `allocator.rs`:

```rust
// in src/allocator.rs

use bump::BumpAllocator;

#[global_allocator]
static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::new());
```

Here it becomes important that I declared `BumpAllocator::new` and `Locked::new` as [`const` functions](https://doc.rust-lang.org/reference/items/functions.html#const-functions). If they were normal functions, a compilation error would occur because the initialization expression of a `static` must be evaluable at compile time.

I didn’t need to change the `ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE)` call in the `init_heap` function because the bump allocator provides the same interface as the allocator provided by the `linked_list_allocator`.

Now the kernel uses the bump allocator! Everything still worked, including the `heap_allocation` tests:

```
> cargo test --test heap_allocation
[…]
Running 3 tests
simple_allocation... [ok]
large_vec... [ok]
many_boxes... [ok]
```

---

## Limitations and Trade-offs

### What This Allocator Can't Do

|Limitation|Why|
|---|---|
|Free individual allocations|Bump allocator only tracks count, not positions|
|Handle non-LIFO deallocation|Resetting only works when all freed|
|Handle large allocations|No fallback mechanism|
|Grow the heap|Fixed size at initialization|

### When This Allocator Works Well

| Scenario                   | Why It's Good                  |
| -------------------------- | ------------------------------ |
| Boot-time initialization   | Allocate once, never free      |
| Temporary phase of kernel  | Allocate then free all at once |
| Single-threaded early init | No lock contention             |
| Testing/demo allocator     | Simple and predictable         |

---

