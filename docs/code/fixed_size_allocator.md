
#### List Node

I started the implementation by creating a `ListNode` type in a new `allocator::fixed_size_block` module:

```rust
// in src/allocator.rs

pub mod fixed_size_block;
```

```rust
// in src/allocator/fixed_size_block.rs

struct ListNode {
    next: Option<&'static mut ListNode>,
}
```

This type is similar to the `ListNode` type of the [linked list allocator implementation](code/linked_list_allocator), with the difference that I didn’t have a `size` field. It isn’t needed because every block in a list has the same size with the fixed-size block allocator design.

## Why a Fixed-Size Block Allocator?

### Problems with Previous Allocators

|Allocator|Problem|
|---|---|
|**Bump Allocator**|Cannot free individual allocations; only resets when all memory freed|
|**Linked List**|O(n) allocation time; must search through list of mixed sizes|

### The Insight

Most kernel allocations fall into a few common sizes:

- Process control blocks (~1KB)
    
- File descriptors (~64 bytes)
    
- Socket buffers (~256 bytes)
    
- Page table entries (8 bytes)
    

If we pre-organize memory by size, we get:

- **O(1) allocation** (just pop from the appropriate free list)
    
- **O(1) deallocation** (just push back to the appropriate free list)
    
- **No fragmentation** within each size class

---
## The ListNode Structure

```rust

struct ListNode {
    next: Option<&'static mut ListNode>,
}
```

### Comparison with Linked List Allocator's ListNode

|Field|Linked List Allocator|Fixed-Size Block|
|---|---|---|
|`size`|Present (regions vary in size)|**Absent** (all blocks in a list are same size)|
|`next`|Present|Present|

### Why No Size Field?

In a linked list allocator, each free region can be a different size. The `size` field tells us how large that region is.

In a fixed-size block allocator, **each free list contains blocks of identical size**. The size is known from which list the block belongs to, not from the block itself. This saves 8 bytes per block!

**Visual:**

```text

List for 32-byte blocks:
┌────────────┐    ┌────────────┐    ┌────────────┐
│ next ──────┼───→│ next ──────┼───→│ next ──────┼───→ None
│ (no size!) │    │ (no size!) │    │ (no size!) │
└────────────┘    └────────────┘    └────────────┘
Each block is exactly 32 bytes total
List for 64-byte blocks:
┌────────────┐    ┌────────────┐
│ next ──────┼───→│ next ──────┼───→ None
└────────────┘    └────────────┘
Each block is exactly 64 bytes total
```

#### Block Sizes

Next, I defined a constant `BLOCK_SIZES` slice with the block sizes used for the implementation:

```rust
// in src/allocator/fixed_size_block.rs

/// The block sizes to use.
///
/// The sizes must each be power of 2 because they are also used as
/// the block alignment (alignments must be always powers of 2).
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
```

As block sizes, I used powers of 2, starting from 8 up to 2048. I didn’t define any block sizes smaller than 8 because each block must be capable of storing a 64-bit pointer to the next block when freed. For allocations greater than 2048 bytes, I will fall back to a linked list allocator.

To simplify the implementation, I defined the size of a block as its required alignment in memory. So a 16-byte block is always aligned on a 16-byte boundary and a 512-byte block is aligned on a 512-byte boundary. Since alignments always need to be powers of 2, this rules out any other block sizes. If I need block sizes that are not powers of 2 in the future, I can still adjust the implementation for this (e.g., by defining a second `BLOCK_ALIGNMENTS` array).

### Why Powers of Two?

|Reason|Explanation|
|---|---|
|**Alignment requirements**|Each block size equals its alignment; alignments must be powers of two|
|**Simple calculation**|Finding the next size is just doubling|
|**Hardware optimization**|Many CPUs prefer power-of-two alignments|
|**Minimal waste**|Powers of two provide good coverage without too many sizes|

### Why Start at 8 Bytes?

- Each block must store a `next` pointer (8 bytes on 64-bit)
    
- Blocks smaller than 8 bytes couldn't store their own free list pointer
    
- 8 bytes is the minimum usable block size
    

### Why Stop at 2048 Bytes (2KB)?

- 2KB is a reasonable maximum for "small" allocations
    
- Larger allocations go to the fallback allocator
    
- Prevents the fixed-size allocator from wasting too much memory on large blocks
    
- Can be adjusted based on system needs
    

### Size Coverage Example

```text

Request: 5 bytes  → 8 byte block   (3 bytes waste)
Request: 9 bytes  → 16 byte block  (7 bytes waste)
Request: 50 bytes → 64 byte block  (14 bytes waste)
Request: 100 bytes → 128 byte block (28 bytes waste)
Request: 2000 bytes → 2048 byte block (48 bytes waste)
Request: 3000 bytes → Fallback allocator

```

The waste (internal fragmentation) is bounded by less than 100% of the request size.

---
#### The Allocator Type

Using the `ListNode` type and the `BLOCK_SIZES` slice, I then defined the allocator type:

```rust
// in src/allocator/fixed_size_block.rs

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}
```

The `list_heads` field is an array of `head` pointers, one for each block size. This is implemented by using the `len()` of the `BLOCK_SIZES` slice as the array length. As a fallback allocator for allocations larger than the largest block size, I used the allocator provided by the `linked_list_allocator`. I could also use the `LinkedListAllocator` I implemented myself instead, but it has the disadvantage that it does not [merge freed blocks](https://os.phil-opp.com/allocator-designs/#merging-freed-blocks).

### Field 1: `list_heads` Array

This is an array of head pointers, one for each block size.

```rust

list_heads[0] → free list for 8-byte blocks
list_heads[1] → free list for 16-byte blocks
list_heads[2] → free list for 32-byte blocks
...
list_heads[8] → free list for 2048-byte blocks
```

**Type: `Option<&'static mut ListNode>`**

- `Some(node)` means the free list has at least one block
    
- `None` means the free list is empty
    

**Why `'static` lifetime?**

- These blocks live in the heap for the entire program
    
- References remain valid forever
    

### Field 2: `fallback_allocator`

A general-purpose allocator for:

- Allocations larger than 2048 bytes
    
- Allocations whose size isn't in `BLOCK_SIZES`
    
- Initial allocation of new blocks for each size class
    

**Why linked_list_allocator::Heap?**

- It's a proven, production-ready allocator
    
- It properly merges adjacent free blocks
    
- We could use our own implementation, but merging would need to be added
    

### Visual Representation

```text

FixedSizeBlockAllocator
│
├── list_heads[0] (8 bytes)   ──→ [Block] ──→ [Block] ──→ None
├── list_heads[1] (16 bytes)  ──→ None (empty)
├── list_heads[2] (32 bytes)  ──→ [Block] ──→ None
├── list_heads[3] (64 bytes)  ──→ None
├── list_heads[4] (128 bytes) ──→ [Block] ──→ [Block] ──→ [Block] ──→ None
├── list_heads[5] (256 bytes) ──→ None
├── list_heads[6] (512 bytes) ──→ [Block] ──→ None
├── list_heads[7] (1024 bytes)──→ None
├── list_heads[8] (2048 bytes)──→ [Block] ──→ None
│
└── fallback_allocator (for large allocations)
```

---

For constructing a `FixedSizeBlockAllocator`, I provided the same `new` and `init` functions that I implemented for the other allocator types too:

```rust
// in src/allocator/fixed_size_block.rs

impl FixedSizeBlockAllocator {
    /// Creates an empty FixedSizeBlockAllocator.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe { self.fallback_allocator.init(heap_start, heap_size); }
    }
}
```

The `new` function just initializes the `list_heads` array with empty nodes and creates an [`empty`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html#method.empty) linked list allocator as `fallback_allocator`. The `EMPTY` constant is needed to tell the Rust compiler that I want to initialize the array with a constant value. Initializing the array directly as `[None; BLOCK_SIZES.len()]` does not work, because then the compiler requires `Option<&'static mut ListNode>` to implement the `Copy` trait, which it does not. This is a current limitation of the Rust compiler, which might go away in the future.

The unsafe `init` function only calls the [`init`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html#method.init) function of the `fallback_allocator` without doing any additional initialization of the `list_heads` array. Instead, I will initialize the lists lazily on `alloc` and `dealloc` calls.

For convenience, I also created a private `fallback_alloc` method that allocates using the `fallback_allocator`:

```rust
// in src/allocator/fixed_size_block.rs

use alloc::alloc::Layout;
use core::ptr;

impl FixedSizeBlockAllocator {
    /// Allocates using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}
```

The [`Heap`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html) type of the `linked_list_allocator` crate does not implement [`GlobalAlloc`](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) (as it’s [not possible without locking](https://os.phil-opp.com/allocator-designs/#globalalloc-and-mutability)). Instead, it provides an [`allocate_first_fit`](https://docs.rs/linked_list_allocator/0.9.0/linked_list_allocator/struct.Heap.html#method.allocate_first_fit) method that has a slightly different interface. Instead of returning a `*mut u8` and using a null pointer to signal an error, it returns a `Result<NonNull<u8>, ()>`. The [`NonNull`](https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html) type is an abstraction for a raw pointer that is guaranteed to not be a null pointer. By mapping the `Ok` case to the [`NonNull::as_ptr`](https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.as_ptr) method and the `Err` case to a null pointer, I can easily translate this back to a `*mut u8` type.

### The `new()` Constructor

```rust

pub const fn new() -> Self {
    const EMPTY: Option<&'static mut ListNode> = None;
    FixedSizeBlockAllocator {
        list_heads: [EMPTY; BLOCK_SIZES.len()],
        fallback_allocator: linked_list_allocator::Heap::empty(),
    }
}
```

**Why the `EMPTY` constant trick?**

```rust

// This doesn't work:
list_heads: [None; BLOCK_SIZES.len()]  // ERROR: Option doesn't implement Copy
// This works:
const EMPTY: Option<&'static mut ListNode> = None;
list_heads: [EMPTY; BLOCK_SIZES.len()]  // OK: using a constant
```

The Rust compiler needs to copy the initial value for each array element. `Option` doesn't implement the `Copy` trait, so it can't be copied. By using a constant, we tell the compiler to reuse the same value (constants are duplicated by value in a way that works even without `Copy`).

**Why `const fn`?**

- Global allocator static must be initialized at compile time
    
- Allocator starts with empty free lists
    
- Fallback allocator starts empty (will be initialized later)
    

### The `init()` Method

```rust

pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
    unsafe { self.fallback_allocator.init(heap_start, heap_size); }
}
```


**Wait, where's the fixed-size block initialization?**

The fixed-size block allocator starts with **empty free lists**! No blocks are pre-allocated.

**How does it get blocks?**

- When you allocate a 64-byte block for the first time, the free list is empty
    
- The allocator asks the fallback allocator for a fresh 64-byte block
    
- That block is returned directly to the user
    
- When the user frees it, it goes into the free list for future allocations
    

This is called **lazy initialization** - blocks are created only when needed.

**Why lazy initialization?**

- Saves memory (don't pre-allocate blocks that might never be used)
    
- Adapts to actual usage patterns
    
- Simpler implementation

---
#### Calculating the List Index

Before I implemented the `GlobalAlloc` trait, I defined a `list_index` helper function that returns the lowest possible block size for a given [`Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html):

```rust
// in src/allocator/fixed_size_block.rs

/// Choose an appropriate block size for the given layout.
///
/// Returns an index into the `BLOCK_SIZES` array.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}
```

The block must have at least the size and alignment required by the given `Layout`. Since I defined that the block size is also its alignment, this means that the `required_block_size` is the [maximum](https://doc.rust-lang.org/core/cmp/trait.Ord.html#method.max) of the layout’s [`size()`](https://doc.rust-lang.org/core/alloc/struct.Layout.html#method.size) and [`align()`](https://doc.rust-lang.org/core/alloc/struct.Layout.html#method.align) attributes. To find the next-larger block in the `BLOCK_SIZES` slice, I first use the [`iter()`](https://doc.rust-lang.org/std/primitive.slice.html#method.iter) method to get an iterator and then the [`position()`](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.position) method to find the index of the first block that is at least as large as the `required_block_size`.

Note that I didn’t return the block size itself, but the index into the `BLOCK_SIZES` slice. The reason is that I want to use the returned index as an index into the `list_heads` array.

### Step-by-Step Logic

#### Step 1: Calculate Required Block Size

```rust

let required_block_size = layout.size().max(layout.align());
```

**Why take the maximum of size and alignment?**

Because we defined that block size = block alignment. The block must be:

1. Large enough to hold the data (`size`)
    
2. Properly aligned for the data type (`align`)
    

**Example:**

```text

Request: 32 bytes, align 16
required_block_size = max(32, 16) = 32
Request: 8 bytes, align 32 (a weird type with high alignment)
required_block_size = max(8, 32) = 32  // Need 32-byte block for alignment!
```

#### Step 2: Find the Smallest Fitting Block Size

```rust

BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
```

`position()` returns the index of the first element that satisfies the condition.

**Examples:**

```text

required = 5   → find s >= 5  → index 0 (8 bytes)
required = 8   → find s >= 8  → index 0 (8 bytes)
required = 9   → find s >= 9  → index 1 (16 bytes)
required = 32  → find s >= 32 → index 2 (32 bytes)
required = 33  → find s >= 33 → index 3 (64 bytes)
required = 2048 → find s >= 2048 → index 8 (2048 bytes)
required = 3000 → find s >= 3000 → None (use fallback)
```

---
#### Implementing `GlobalAlloc`

The last step was to implement the `GlobalAlloc` trait:

```rust
// in src/allocator/fixed_size_block.rs

use super::Locked;
use alloc::alloc::GlobalAlloc;

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!();
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!();
    }
}
```

Like for the other allocators, I didn’t implement the `GlobalAlloc` trait directly for the allocator type, but used the [`Locked` wrapper](https://os.phil-opp.com/allocator-designs/#a-locked-wrapper-type) to add synchronized interior mutability. 

### `alloc`

The implementation of the `alloc` method looks like this:

```rust
// in `impl` block in src/allocator/fixed_size_block.rs

unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let mut allocator = self.lock();
    match list_index(&layout) {
        Some(index) => {
            match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    // no block exists in list => allocate new block
                    let block_size = BLOCK_SIZES[index];
                    // only works if all block sizes are a power of 2
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align)
                        .unwrap();
                    allocator.fallback_alloc(layout)
                }
            }
        }
        None => allocator.fallback_alloc(layout),
    }
}
```

### Step-by-Step Execution

#### Step 1: Lock the Allocator

```rust

let mut allocator = self.lock();
```

- Acquires spinlock (thread safety)
    
- Returns mutable reference to inner allocator
    

#### Step 2: Find Appropriate Size Class

```rust

match list_index(&layout) {
```

**Case A: No fitting size class (`None`)**

```rust

None => allocator.fallback_alloc(layout)
```

- Request is larger than 2048 bytes
    
- Or alignment requirement is higher than any block size
    
- Forward directly to fallback allocator
    

**Case B: Fitting size class found (`Some(index)`)**

#### Step 3: Try to Pop from Free List

```rust

match allocator.list_heads[index].take() {
```

`take()` does two things:

1. Takes ownership of the `Some` value
    
2. Leaves `None` in its place
    

**Subcase B1: Free list has a block (`Some(node)`)**

```rust

allocator.list_heads[index] = node.next.take();
node as *mut ListNode as *mut u8
```

**Visual before:**

```text

list_heads[index] ──> [Block A] ──> [Block B] ──> None
```

**After `take()` on list_heads:**

```text

node = Block A
list_heads[index] = None
[Block A] ──> [Block B] ──> None
```

**After `node.next.take()`:**

```text

node.next = None (Block A's next becomes None)
next = Block B (saved)
```

**After setting list_heads[index] = next:**

```text

list_heads[index] ──> [Block B] ──> None
```

**Result:** Block A removed from list and returned to user.

**Subcase B2: Free list is empty (`None`)**

```rust

let block_size = BLOCK_SIZES[index];
let block_align = block_size;
let layout = Layout::from_size_align(block_size, block_align).unwrap();
allocator.fallback_alloc(layout)
```

**What happens:**

1. Get block size (e.g., 64 bytes)
    
2. Create layout with same size and alignment
    
3. Ask fallback allocator for a fresh block
    
4. Return that block directly (don't put in free list yet)
    

**Why not put it in free list now?**  
The block is allocated immediately to the user. It only goes to the free list when deallocated..

---
#### [🔗](https://os.phil-opp.com/allocator-designs/#dealloc)`dealloc`

The implementation of the `dealloc` method looks like this:

```rust
// in src/allocator/fixed_size_block.rs

use core::{mem, ptr::NonNull};

// inside the `unsafe impl GlobalAlloc` block

unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    let mut allocator = self.lock();
    match list_index(&layout) {
        Some(index) => {
            let new_node = ListNode {
                next: allocator.list_heads[index].take(),
            };
            // verify that block has size and alignment required for storing node
            assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
            assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
            let new_node_ptr = ptr as *mut ListNode;
            unsafe {
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
        }
        None => {
            let ptr = NonNull::new(ptr).unwrap();
            unsafe {
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}
```

### Step-by-Step Execution

#### Step 1: Lock and Find Size Class

Same as allocation.

#### Step 2: Handle Fallback Case

```rust

None => {
    let ptr = NonNull::new(ptr).unwrap();
    allocator.fallback_allocator.deallocate(ptr, layout);
}
```

- Convert `*mut u8` to `NonNull<u8>` (non-null pointer type)
    
- Return memory to fallback allocator
    

#### Step 3: Handle Fixed-Size Case

```rust

let new_node = ListNode {
    next: allocator.list_heads[index].take(),
};
```

**Create new ListNode:**

- Set its `next` to current head of free list
    
- `take()` removes head and gives us the old head
    

**Visual before:**

```text

list_heads[index] ──> [Block X] ──> [Block Y] ──> None
```


**After creating new_node:**

```text

new_node.next = Block X
list_heads[index] = None
[Block X] ──> [Block Y] ──> None
```


#### Step 4: Assertions (Safety Checks)

```rust

assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
```

**First assertion:** The block must be large enough to store a ListNode.  
**Second assertion:** The block's alignment must be sufficient for a ListNode.

These should always pass because:

- Smallest block size is 8 bytes, same as pointer size
    
- Block alignment is block size (power of two) ≥ ListNode alignment
    

#### Step 5: Write Node to Freed Memory

```rust

let new_node_ptr = ptr as *mut ListNode;
new_node_ptr.write(new_node);
```

**Important:** This writes the ListNode **into the freed memory block** itself!

**Visual:**

```text

Freed block at address ptr:
Before: [user data (whatever was there)]
After:  [next pointer] [unused space...]
         ↑
    This is the ListNode

```

#### Step 6: Update List Head

```rust

allocator.list_heads[index] = Some(&mut *new_node_ptr);
```

**Visual after:**

```text

list_heads[index] ──> [new_node (at ptr)] ──> [Block X] ──> [Block Y] ──> None
```

The freed block is now at the front of the free list, ready for future allocations.

There are a few things worth noting:

- I didn’t differentiate between blocks allocated from a block list and blocks allocated from the fallback allocator. This means that new blocks created in `alloc` are added to the block list on `dealloc`, thereby increasing the number of blocks of that size.
- The `alloc` method is the only place where new blocks are created in the implementation. This means that I initially start with empty block lists and only fill these lists lazily when allocations of their block size are performed.
- I didn’t need `unsafe` blocks in `alloc` and `dealloc`, even though I perform some `unsafe` operations. The reason is that Rust currently treats the complete body of unsafe functions as one large `unsafe` block. Since using explicit `unsafe` blocks has the advantage that it’s obvious which operations are unsafe and which are not, there is a [proposed RFC](https://github.com/rust-lang/rfcs/pull/2585) to change this behavior.

### Using it

To use the new `FixedSizeBlockAllocator`, I needed to update the `ALLOCATOR` static in the `allocator` module:

```rust
// in src/allocator.rs

use fixed_size_block::FixedSizeBlockAllocator;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(
    FixedSizeBlockAllocator::new());
```

Since the `init` function behaves the same for all allocators I implemented, I didn’t need to modify the `init` call in `init_heap`.

When I ran the `heap_allocation` tests again, all tests still passed:

```
> cargo test --test heap_allocation
simple_allocation... [ok]
large_vec... [ok]
many_boxes... [ok]
many_boxes_long_lived... [ok]
```

---

