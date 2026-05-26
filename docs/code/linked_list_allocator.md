
### Implementation

---
#### The Allocator Type

I started by creating a private `ListNode` struct in a new `allocator::linked_list` submodule:

```rust
// in src/allocator.rs

pub mod linked_list;
```

```rust
// in src/allocator/linked_list.rs

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}
```

A list node has a `size` field and an optional pointer to the next node, represented by the `Option<&'static mut ListNode>` type. The `&'static mut` type semantically describes an [owned](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html) object behind a pointer. Basically, it’s a [`Box`](https://doc.rust-lang.org/alloc/boxed/index.html) without a destructor that frees the object at the end of the scope.

### Field Explanations

|Field|Purpose|Why It Exists|
|---|---|---|
|`size`|Size of this free memory region in bytes|Know how much memory this node represents|
|`next`|Pointer to the next free node in the list|Create a linked list; `None` means end of list|

### Understanding `&'static mut ListNode`

This type is subtle but clever:

|Component|Meaning|
|---|---|
|`&'static mut`|A mutable reference that lives for the entire program|
|`ListNode`|The type being referenced|

**Why `'static`?**

- The heap memory exists for the entire program duration
    
- References to heap memory must be valid forever
    
- `'static` lifetime expresses this guarantee
    

**Why `mut`?**

- We need to modify the `next` pointer of nodes
    
- When we remove a node from the list, we change what it points to
    

**Why an `Option`?**

- End of list: `None`
    
- Has next node: `Some(&'static mut ListNode)`
    

### The Self-Referential Nature

Here's the brilliant part: **The ListNode is stored inside the free memory region it describes!**

```text

Memory region at address 0x1000, size 4096 bytes:
0x1000: [size: 4096] [next: pointer] ← ListNode lives here
0x1008:                                     |
0x1010:                                     |
...                                         |
0x2000: (end of region)                     |
                                            |
                                            v
                              Points to another ListNode
                              somewhere else in memory
```


**Why is this efficient?**

- No separate metadata structures needed
    
- The free memory itself stores the list nodes
    
- When memory is allocated, the ListNode is overwritten with user data

---

I implemented the following set of methods for `ListNode`:

```rust
// in src/allocator/linked_list.rs

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}
```

The type has a simple constructor function named `new` and methods to calculate the start and end addresses of the represented region. I make the `new` function a [const function](https://doc.rust-lang.org/reference/items/functions.html#const-functions), which will be required later when constructing a static linked list allocator.

### `new()` - Constructor

**Purpose:** Create a new ListNode with given size and no next pointer.

**Why `const fn`?**

- The `head` node is created at compile time
    
- `const fn` allows compile-time evaluation
    

**Why `next: None`?**

- New nodes are added to front of list
    
- The caller will set `next` appropriately
    

### `start_addr()` - Get Region Start

```rust

fn start_addr(&self) -> usize {
    self as *const Self as usize
}
```

**How it works:**

- `self` is a reference to the ListNode
    
- `self as *const Self` converts reference to raw pointer
    
- `as usize` converts pointer to integer address
    

**What this returns:** The memory address where this ListNode lives, which is also the start address of the free region.

### `end_addr()` - Get Region End

```rust

fn end_addr(&self) -> usize {
    self.start_addr() + self.size
}
```

**What this returns:** The address just past the end of the free region. This is a standard "one-past-the-end" pointer.

**Visual:**

```text

start_addr() = 0x1000
size = 4096
end_addr() = 0x1000 + 4096 = 0x2000
[0x1000]============Free Region============[0x2000]
 ↑                                           ↑
start_addr()                        end_addr() (exclusive)
```

---

With the `ListNode` struct as a building block, I then created the `LinkedListAllocator` struct:

```rust
// in src/allocator/linked_list.rs

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Creates an empty LinkedListAllocator.
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        todo!();
    }
}
```

The struct contains a `head` node that points to the first heap region. I are only interested in the value of the `next` pointer, so I set the `size` to 0 in the `ListNode::new` function. Making `head` a `ListNode` instead of just a `&'static mut ListNode` has the advantage that the implementation of the `alloc` method will be simpler.

Like for the bump allocator, the `new` function doesn’t initialize the allocator with the heap bounds. In addition to maintaining API compatibility, the reason is that the initialization routine requires writing a node to the heap memory, which can only happen at runtime. The `new` function, however, needs to be a [`const` function](https://doc.rust-lang.org/reference/items/functions.html#const-functions) that can be evaluated at compile time because it will be used for initializing the `ALLOCATOR` static. For this reason, I again provide a separate, non-constant `init` method.

The `init` method uses an `add_free_region` method, whose implementation will be shown in a moment. For now, I use the [`todo!`](https://doc.rust-lang.org/core/macro.todo.html) macro to provide a placeholder implementation that always panics.

```rust

pub struct LinkedListAllocator {
    head: ListNode,
}

```

### Why a Dummy Head Node?

The `head` node is a **sentinel** (dummy) node that simplifies list operations.

**Without sentinel (using Option):**

```rust

struct LinkedListAllocator {
    head: Option<&'static mut ListNode>,
}
// Adding to empty list:
match &mut self.head {
    None => self.head = Some(new_node),
    Some(h) => {
        new_node.next = self.head.take();
        self.head = Some(new_node);
    }
}
```

**With sentinel (our implementation):**

```rust

struct LinkedListAllocator {
    head: ListNode,  // Always exists, size=0, next=initially None
}
// Adding to any list (empty or not):
new_node.next = self.head.next.take();
self.head.next = Some(new_node);
```

**Advantages of sentinel:**

- No special cases for empty list
    
- Simpler code (no `Option` checks for head)
    
- The `head` node never changes, only its `next` pointer
    

**The sentinel's fields:**

- `size: 0` (doesn't represent any real memory)
    
- `next: None` initially, then points to first real free region
    

**Visual:**

```text

head (sentinel, size=0)
  │
  │ next
  ▼
┌─────────┐    ┌─────────┐    ┌─────────┐
│ ListNode│───→│ ListNode│───→│ ListNode│───→ None
│ size=4KB│    │ size=2KB│    │ size=8KB│
└─────────┘    └─────────┘    └─────────┘
```

---

#### The `add_free_region` Method

The `add_free_region` method provides the fundamental _push_ operation on the linked list. I currently only call this method from `init`, but it will also be the central method in the `dealloc` implementation. The `dealloc` method is called when an allocated memory region is freed again. To keep track of this freed memory region, I want to push it to the linked list.

The implementation of the `add_free_region` method looks like this:

```rust
// in src/allocator/linked_list.rs

use super::align_up;
use core::mem;

impl LinkedListAllocator {
    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed region is capable of holding ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // create a new list node and append it at the start of the list
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(node);
            self.head.next = Some(&mut *node_ptr)
        }
    }
}
```

The method takes the address and size of a memory region as an argument and adds it to the front of the list. First, it ensures that the given region has the necessary size and alignment for storing a `ListNode`. Then it creates the node and inserts it into the list through the following steps:

![](https://os.phil-opp.com/allocator-designs/linked-list-allocator-push.svg)

Step 0 shows the state of the heap before `add_free_region` is called. In step 1, the method is called with the memory region marked as `freed` in the graphic. After the initial checks, the method creates a new `node` on its stack with the size of the freed region. It then uses the [`Option::take`](https://doc.rust-lang.org/core/option/enum.Option.html#method.take) method to set the `next` pointer of the node to the current `head` pointer, thereby resetting the `head` pointer to `None`.

In step 2, the method writes the newly created `node` to the beginning of the freed memory region through the [`write`](https://doc.rust-lang.org/std/primitive.pointer.html#method.write) method. It then points the `head` pointer to the new node. The resulting pointer structure looks a bit chaotic because the freed region is always inserted at the beginning of the list, but if I follow the pointers, I see that each free region is still reachable from the `head` pointer.

### Step-by-Step Logic

#### Step 1: Validate the Region

```rust

assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
assert!(size >= mem::size_of::<ListNode>());
```


**First assertion:** The region must be properly aligned for a ListNode

- ListNode contains a `usize` (8 bytes) and a pointer (8 bytes)
    
- Typical alignment is 8 or 16 bytes
    
- If address isn't aligned, writing a ListNode could cause CPU faults
    

**Second assertion:** The region must be large enough to hold a ListNode

- We store the ListNode at the start of the region
    
- If region is smaller than ListNode, we can't track it
    
- Minimum region size is typically 16-32 bytes
    

#### Step 2: Create the Node Structure

```rust

let mut node = ListNode::new(size);
```

This creates a ListNode in **stack memory** (temporary). We'll copy it to the heap.

#### Step 3: Insert at Front of List

```rust

node.next = self.head.next.take();
```

**What `take()` does:** Replaces `self.head.next` with `None` and returns the old value.

**Before insertion:**

```text

head ──→ [Node A] ──→ [Node B] ──→ None

```

**After `node.next = self.head.next.take()`:**

```text

node.next ──→ [Node A] ──→ [Node B] ──→ None
head.next = None
```

#### Step 4: Write Node to Memory

```rust

let node_ptr = addr as *mut ListNode;
node_ptr.write(node);
self.head.next = Some(&mut *node_ptr)
```

**`node_ptr.write(node)`:** Copies the ListNode from stack to heap at `addr`

**`&mut *node_ptr`:** Converts raw pointer back to safe reference

**After insertion:**

```text

head ──→ [New Node] ──→ [Node A] ──→ [Node B] ──→ None
```


### Why This Works (The Magic of Embedded Nodes)

The ListNode is written into the **free memory region itself**:

```text

Free region at 0x1000, size 4096:
After add_free_region:
0x1000: [size: 4096] [next: pointer] ← ListNode stored here
0x1008: (free space continues)
...
0x2000: end of region
```

The ListNode and the free region are the same memory!
When this region is allocated, the ListNode is overwritten.

#### The `find_region` Method

The second fundamental operation on a linked list is finding an entry and removing it from the list. This is the central operation needed for implementing the `alloc` method. I implemented the operation as a `find_region` method in the following way:

```rust
// in src/allocator/linked_list.rs

impl LinkedListAllocator {
    /// Looks for a free region with the given size and alignment and removes
    /// it from the list.
    ///
    /// Returns a tuple of the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize)
        -> Option<(&'static mut ListNode, usize)>
    {
        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;
        // look for a large enough memory region in linked list
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // region suitable for allocation -> remove node from list
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // region not suitable -> continue with next region
                current = current.next.as_mut().unwrap();
            }
        }

        // no suitable region found
        None
    }
}
```

The method uses a `current` variable and a [`while let` loop](https://doc.rust-lang.org/reference/expressions/loop-expr.html#while-let-patterns) to iterate over the list elements. At the beginning, `current` is set to the (dummy) `head` node. On each iteration, it is then updated to the `next` field of the current node (in the `else` block). If the region is suitable for an allocation with the given size and alignment, the region is removed from the list and returned together with the `alloc_start` address.

When the `current.next` pointer becomes `None`, the loop exits. This means I iterated over the whole list but found no region suitable for an allocation. In that case, I return `None`. Whether a region is suitable is checked by the `alloc_from_region` function, whose implementation will be shown in a moment.

Let’s take a more detailed look at how a suitable region is removed from the list:

![](https://os.phil-opp.com/allocator-designs/linked-list-allocator-remove-region.svg)

Step 0 shows the situation before any pointer adjustments. The `region` and `current` regions and the `region.next` and `current.next` pointers are marked in the graphic. In step 1, both the `region.next` and `current.next` pointers are reset to `None` by using the [`Option::take`](https://doc.rust-lang.org/core/option/enum.Option.html#method.take) method. The original pointers are stored in local variables called `next` and `ret`.

In step 2, the `current.next` pointer is set to the local `next` pointer, which is the original `region.next` pointer. The effect is that `current` now directly points to the region after `region`, so that `region` is no longer an element of the linked list. The function then returns the pointer to `region` stored in the local `ret` variable.

### Step-by-Step Logic

#### Step 1: Initialize Search

```rust

let mut current = &mut self.head;
```

`current` always points to the node **before** the node we're examining. This is a classic linked list trick that makes removal easy.

**Visual:**

```text

current = head (sentinel)
  │
  ▼
head ──→ [Node A] ──→ [Node B] ──→ None
         ↑
    region (current.next)
```

#### Step 2: Iterate Through List

```rust

while let Some(ref mut region) = current.next {
    // Check if region works
}

```

This pattern:

- `current.next` is `Option<&'static mut ListNode>`
    
- `Some(ref mut region)` gives mutable reference to the node
    
- Loop continues as long as there's a next node
    

#### Step 3: Check Region Suitability

```rust

if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
    // Region works
}
```

Call `alloc_from_region` to see if this region can satisfy the allocation. If yes, it returns the start address where allocation should go.

#### Step 4: Remove Region from List

```rust

let next = region.next.take();           // Save what region pointed to
let ret = Some((current.next.take().unwrap(), alloc_start));
current.next = next;                      // Bypass the removed region

```

**Before removal:**

```text

current ──→ [Region] ──→ [Next Node] ──→ ...
```

**After `next = region.next.take()`:**

```text

region.next = None
next points to [Next Node]
```

**After `current.next.take()`:**

```text

current.next = None
Ret has the region
```

**After `current.next = next`:**

```text

current ──→ [Next Node] ──→ ...
(Region is removed)

```
#### Step 5: Handle Failure

```rust
} else {
    current = current.next.as_mut().unwrap();
}
```

If region isn't suitable, move `current` to this region and continue searching.

##### The `alloc_from_region` Function

The `alloc_from_region` function returns whether a region is suitable for an allocation with a given size and alignment. It is defined like this:

```rust
// in src/allocator/linked_list.rs

impl LinkedListAllocator {
    /// Try to use the given region for an allocation with given size and
    /// alignment.
    ///
    /// Returns the allocation start address on success.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize)
        -> Result<usize, ()>
    {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // rest of region too small to hold a ListNode (required because the
            // allocation splits the region in a used and a free part)
            return Err(());
        }

        // region suitable for allocation
        Ok(alloc_start)
    }
}
```

First, the function calculates the start and end address of a potential allocation, using the `align_up` function I defined earlier and the [`checked_add`](https://doc.rust-lang.org/std/primitive.usize.html#method.checked_add) method. If an overflow occurs or if the end address is behind the end address of the region, the allocation doesn’t fit in the region and I return an error.

The function performs a less obvious check after that. This check is necessary because most of the time an allocation does not fit a suitable region perfectly, so that a part of the region remains usable after the allocation. This part of the region must store its own `ListNode` after the allocation, so it must be large enough to do so. The check verifies exactly that: either the allocation fits perfectly (`excess_size == 0`) or the excess size is large enough to store a `ListNode`.

### Step-by-Step Logic

#### Step 1: Calculate Potential Allocation Boundaries

```rust

let alloc_start = align_up(region.start_addr(), align);
let alloc_end = alloc_start.checked_add(size).ok_or(())?;

```

**Example:**

- Region starts at 0x1000, size 4096 (ends at 0x2000)
    
- Request: 100 bytes with alignment 8
    
- `alloc_start = align_up(0x1000, 8) = 0x1000`
    
- `alloc_end = 0x1000 + 100 = 0x1064`
    

#### Step 2: Check If Allocation Fits

```rust

if alloc_end > region.end_addr() {
    return Err(());
}
```

- If `alloc_end` goes past region end, region is too small
    
- In example: 0x1064 ≤ 0x2000 → passes
    

#### Step 3: Check If Remaining Space Can Store ListNode

```rust

let excess_size = region.end_addr() - alloc_end;
if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
    return Err(());
}
```

**This is the subtle but critical check!**

When we allocate part of a region, the leftover space must be able to store its own ListNode. If it's too small for a ListNode, we can't track it as a free region.

**Example where check fails:**

- Region: 0x1000 to 0x2000 (4096 bytes)
    
- Request: 4088 bytes with alignment 8
    
- `alloc_start = 0x1000`
    
- `alloc_end = 0x1000 + 4088 = 0x1FF8`
    
- `excess_size = 0x2000 - 0x1FF8 = 8 bytes`
    
- ListNode size is 16 bytes → 8 < 16 → REJECT!
    

**Why reject?**  
If we allocate here, we'd have 8 bytes leftover. But we can't put a ListNode in 8 bytes (needs 16 bytes). Those 8 bytes become unusable forever (fragmentation). Better to not use this region and find a better fit.

**When check passes:**

- Perfect fit: `excess_size == 0` → use entire region
    
- Large leftover: `excess_size >= sizeof(ListNode)` → split region

---

### Implementing `GlobalAlloc`

With the fundamental operations provided by the `add_free_region` and `find_region` methods, I then implemented the `GlobalAlloc` trait. As with the bump allocator, I didn’t implement the trait directly for the `LinkedListAllocator` but only for a wrapped `Locked<LinkedListAllocator>`. The [`Locked` wrapper](https://os.phil-opp.com/allocator-designs/#a-locked-wrapper-type) adds interior mutability through a spinlock, which allowed me to modify the allocator instance even though the `alloc` and `dealloc` methods only take `&self` references.

The implementation looks like this:

```rust
// in src/allocator/linked_list.rs

use super::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // perform layout adjustments
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // perform layout adjustments
        let (size, _) = LinkedListAllocator::size_align(layout);

        unsafe { self.lock().add_free_region(ptr as usize, size) }
    }
}
```

Let’s start with the `dealloc` method because it is simpler: First, it performs some layout adjustments, which I will explain in a moment. Then, it retrieves a `&mut LinkedListAllocator` reference by calling the [`Mutex::lock`](https://docs.rs/spin/0.5.0/spin/struct.Mutex.html#method.lock) function on the [`Locked` wrapper](https://os.phil-opp.com/allocator-designs/#a-locked-wrapper-type). Lastly, it calls the `add_free_region` function to add the deallocated region to the free list.

The `alloc` method is a bit more complex. It starts with the same layout adjustments and also calls the [`Mutex::lock`](https://docs.rs/spin/0.5.0/spin/struct.Mutex.html#method.lock) function to receive a mutable allocator reference. Then it uses the `find_region` method to find a suitable memory region for the allocation and remove it from the list. If this doesn’t succeed and `None` is returned, it returns `null_mut` to signal an error as there is no suitable memory region.

In the success case, the `find_region` method returns a tuple of the suitable region (no longer in the list) and the start address of the allocation. Using `alloc_start`, the allocation size, and the end address of the region, it calculates the end address of the allocation and the excess size again. If the excess size is not null, it calls `add_free_region` to add the excess size of the memory region back to the free list. Finally, it returns the `alloc_start` address casted as a `*mut u8` pointer.

#### Layout Adjustments

So what are these layout adjustments that I made at the beginning of both `alloc` and `dealloc`? They ensure that each allocated block is capable of storing a `ListNode`. This is important because the memory block is going to be deallocated at some point, where I want to write a `ListNode` to it. If the block is smaller than a `ListNode` or does not have the correct alignment, undefined behavior can occur.

The layout adjustments are performed by the `size_align` function, which is defined like this:

```rust
// in src/allocator/linked_list.rs

impl LinkedListAllocator {
    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}
```

First, the function uses the [`align_to`](https://doc.rust-lang.org/core/alloc/struct.Layout.html#method.align_to) method on the passed [`Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html) to increase the alignment to the alignment of a `ListNode` if necessary. It then uses the [`pad_to_align`](https://doc.rust-lang.org/core/alloc/struct.Layout.html#method.pad_to_align) method to round up the size to a multiple of the alignment to ensure that the start address of the next memory block will have the correct alignment for storing a `ListNode` too. In the second step, it uses the [`max`](https://doc.rust-lang.org/std/cmp/trait.Ord.html#method.max) method to enforce a minimum allocation size of `mem::size_of::<ListNode>`. This way, the `dealloc` function can safely write a `ListNode` to the freed memory block.

### The `size_align()` Helper

```rust

fn size_align(layout: Layout) -> (usize, usize) {
    let layout = layout
        .align_to(mem::align_of::<ListNode>())
        .expect("adjusting alignment failed")
        .pad_to_align();
    let size = layout.size().max(mem::size_of::<ListNode>());
    (size, layout.align())
}
```

**Why adjust layout?**  
Every allocated block might be freed later. When freed, we need to write a ListNode at its start. Therefore, every block must be:

1. Large enough to hold a ListNode
    
2. Properly aligned for a ListNode
    

**Step-by-step:**

|Step|What it does|Example|
|---|---|---|
|Original layout|User request|10 bytes, align 1|
|`align_to(ListNode alignment)`|Increase alignment|align 1 → align 8|
|`pad_to_align()`|Round size to alignment multiple|10 → 16 bytes|
|`max(size_of::<ListNode>())`|Ensure minimum size|16 bytes (already ≥ 16)|
|Final|Safe for ListNode storage|16 bytes, align 8|

### The `alloc()` Method

```rust

unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let (size, align) = LinkedListAllocator::size_align(layout);
    let mut allocator = self.lock();
    
    if let Some((region, alloc_start)) = allocator.find_region(size, align) {
        let alloc_end = alloc_start.checked_add(size).expect("overflow");
        let excess_size = region.end_addr() - alloc_end;
        
        if excess_size > 0 {
            unsafe {
                allocator.add_free_region(alloc_end, excess_size);
            }
        }
        
        alloc_start as *mut u8
    } else {
        ptr::null_mut()
    }
}
```

**Visual example:**

```text

Initial free list:
head ──→ [Region A: 0x1000-0x3000] ──→ [Region B: 0x5000-0x6000]
Request: 4096 bytes (4KB)
1. find_region finds Region A (size 8192 bytes)
2. alloc_from_region calculates:
   alloc_start = 0x1000 (aligned)
   alloc_end = 0x1000 + 4096 = 0x2000
   excess_size = 0x3000 - 0x2000 = 4096 bytes
3. Region A removed from list
4. Excess added as new region:
   add_free_region(0x2000, 4096)
After allocation:
head ──→ [Excess: 0x2000-0x3000] ──→ [Region B: 0x5000-0x6000]
Allocated: 0x1000-0x2000 (returned to user)
```

### The `dealloc()` Method

```rust

unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    let (size, _) = LinkedListAllocator::size_align(layout);
    unsafe { self.lock().add_free_region(ptr as usize, size) }
}
```

**Why so simple?**

- Deallocation just adds the region back to the free list
    
- No need to merge yet (though real allocators merge adjacent free blocks)
    

**Important:** This implementation doesn't merge adjacent free regions! A production allocator would need to:

1. Insert region in address order (not just at front)
    
2. Merge with previous and next regions if adjacent

---
### Using it

I then updated the `ALLOCATOR` static in the `allocator` module to use the new `LinkedListAllocator`:

```rust
// in src/allocator.rs

use linked_list::LinkedListAllocator;

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> =
    Locked::new(LinkedListAllocator::new());
```

Since the `init` function behaves the same for the bump and linked list allocators, I didn’t need to modify the `init` call in `init_heap`.

When I ran the `heap_allocation` tests again, I saw that all tests passed, including the `many_boxes_long_lived` test that failed with the bump allocator:

```
> cargo test --test heap_allocation
simple_allocation... [ok]
large_vec... [ok]
many_boxes... [ok]
many_boxes_long_lived... [ok]
```

This shows that the linked list allocator is able to reuse freed memory for subsequent allocations.

## Why This Passes the Long-Lived Test

Remember the bump allocator failed `many_boxes_long_lived` because it couldn't reuse memory:

```rust

// Test pattern that failed with bump allocator:
for i in 0..1000 {
    let x = Box::new(i);     // Allocate
    let y = Box::new(i+1);   // Allocate
    // y dropped here (deallocated)
}
// x dropped here (deallocated)
// Next iteration: needs new allocations
```

**Bump allocator behavior:**

```text

Iteration 1: alloc x (0x1000), alloc y (0x1008)
Drop y: allocations=1 (no reset, y's memory lost forever)
Drop x: allocations=0 (reset next=0x1000)
Iteration 2: alloc x (0x1000), alloc y (0x1008)
Works but reuses only because reset happened when all freed
```


**Linked list allocator behavior:**

```text

Iteration 1: alloc x from region A, alloc y from region A
Drop y: add y's region to free list
Drop x: add x's region to free list (list now has both)
Iteration 2: alloc x from region A, alloc y from region A
Reuses same memory! No reset needed.
```

The linked list allocator handles arbitrary allocation/deallocation patterns, not just LIFO.

---
## Key Design Decisions

|Decision|Why|
|---|---|
|**ListNode stored in free memory**|Zero metadata overhead|
|**Sentinel head node**|Simplifies list operations (no empty list special cases)|
|**Add to front (not sorted)**|Simple O(1) insertion; merging would require sorting|
|**Minimum allocation size = sizeof(ListNode)**|Every block must be able to store a ListNode when freed|
|**Split regions on allocation**|Maximizes memory utilization|
|**Don't merge adjacent regions**|Simpler but causes fragmentation (production allocators merge)|
|**First-fit allocation**|Simpler than best-fit; adequate for many cases|

---
## Limitations Compared to Bump Allocator

|Aspect|Bump|Linked List|
|---|---|---|
|Speed|O(1) allocation|O(n) allocation (must search)|
|Deallocation|O(1) (just decrement count)|O(1) (add to front)|
|Memory reuse|Only when all freed|Individual blocks|
|Fragmentation|None|External fragmentation possible|
|Metadata overhead|None|ListNode per free block|

---
## When to Use Each

|Scenario|Best Choice|
|---|---|
|Boot-time initialization|Bump allocator|
|Temporary phase (then free all)|Bump allocator|
|General kernel heap|Linked list (or slab)|
|Long-running system with mixed alloc/free|Linked list with merging|
|Real-time systems|Bump or TLSF (constant time)|

The linked list allocator is a significant step up from the bump allocator, enabling proper memory reuse and making it suitable as a general-purpose kernel heap allocator. While not as fast or sophisticated as production allocators (slab, buddy, jemalloc), it demonstrates the core concepts and works well for learning and simple systems.

---
