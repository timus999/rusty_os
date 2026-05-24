
## Understanding Memory Management: Variables, Allocations, and Rust's GlobalAlloc

This is a comprehensive guide to memory management concepts, from basic variable types to Rust's allocation traits.

---

## Table of Contents

1. Variable Types in Programming
    
    - Local Variables
        
    - Static Variables
        
    - Dynamic Variables
        
2. Rust's Heap Allocation Methods
    
3. The GlobalAlloc Trait
    
4. Putting It All Together
    

---

## Variable Types in Programming

### Local Variables (Stack Allocation)

#### What They Are

Variables allocated on the **stack** with automatic lifetime scoped to their containing block.

#### Characteristics

|Property|Description|
|---|---|
|**Allocation time**|Compile-time known size|
|**Lifetime**|Function/block scope only|
|**Deallocation**|Automatic when scope exits|
|**Speed**|Very fast (just adjust stack pointer)|
|**Size limit**|Stack size limited (typically 1-8 MB)|
|**Memory location**|CPU stack|
**Rust:**

```rust

fn function() {
    let x = 42;           // Local variable (i32 on stack)
    let buffer = [0u8; 256]; // Fixed array on stack
    let point = Point { x: 1, y: 2 }; // Struct on stack
} // Dropped here automatically
```

#### Stack Layout Visualization

```text

High addresses
+------------------+
|  Return address  |
+------------------+
|  Previous frame  |
|     pointer      |
+------------------+
|  Local var: x    |  ← 4 bytes
+------------------+
|  Local var: y    |  ← 4 bytes
+------------------+
|  Array buffer[256]| ← 256 bytes
+------------------+
Low addresses      ← Stack pointer (RSP)
```

#### When to Use Local Variables

- Small, fixed-size data
    
- Temporary values
    
- Data that doesn't outlive the function
    
- Performance-critical code (no allocation overhead)

---

### Static Variables (Static Allocation)

#### What They Are

Variables allocated at **compile time** with lifetime equal to program execution.

#### Characteristics

|Property|Description|
|---|---|
|**Allocation time**|Compile time (in binary)|
|**Lifetime**|Entire program duration|
|**Deallocation**|Never (program exit only)|
|**Speed**|Fastest (fixed address)|
|**Memory location**|Data segment (or BSS for zero-initialized)|
|**Mutability**|Often read-only (const) or carefully synchronized|

#### Types of Static Variables

**1. Global Variables:**

```rust

// Rust
static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
fn increment() {
    GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
}
```

**2. Static Local Variables:**

```rust

use std::sync::atomic::{AtomicU32, Ordering};
fn counter() {
    static CALLS: AtomicU32 = AtomicU32::new(0);
    let calls = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    println!("Called {} times", calls);
}
```

**3. Constants:**

```rust

//- compile-time constant (inlined, no memory address)
const MAX_SIZE: usize = 1024;
```

#### Memory Layout

```text

Executable File Layout:
+------------------+
|   Text segment   | ← Code (read-only)
|   (code)         |
+------------------+
|   Data segment   | ← Initialized statics (.data)
|   (initialized)  |   (global_counter = 0)
+------------------+
|   BSS segment    | ← Zero-initialized statics (.bss)
|   (zero-init)    |   (static int x;)
+------------------+
|   Read-only data | ← Constants, string literals (.rodata)
|   (constants)    |
+------------------+
```

#### When to Use Static Variables

- Configuration data
    
- Global state (carefully!)
    
- String literals
    
- Lookup tables
    
- Singleton patterns
    
- Inter-function shared state (with synchronization)
    

#### Warnings

- **Thread safety**: Mutable statics need synchronization
    
- **Testing difficulty**: Global state persists between tests
    
- **Code coupling**: Makes functions less reusable

---

### Dynamic Variables (Heap Allocation)

#### What They Are

Variables allocated at **runtime** on the **heap** with programmer-controlled lifetime.

#### Characteristics

|Property|Description|
|---|---|
|**Allocation time**|Runtime|
|**Lifetime**|Until explicitly deallocated|
|**Deallocation**|Manual (C) or automatic (Rust via Drop)|
|**Speed**|Slower (requires heap management)|
|**Size limit**|Limited by available RAM|
|**Memory location**|Heap|

#### Basic Example

```rust

// Allocate (Box puts data on heap)
let arr = vec![0; 100];  // Vec allocates heap memory
// Use
// (automatically dropped when arr goes out of scope)
// No manual free needed!
```

---
#### Heap Allocation in Rust

Rust provides several heap-allocated types:

|Type|Description|Allocation Behavior|
|---|---|---|
|`Box<T>`|Single value on heap|Allocates when created, frees when dropped|
|`Vec<T>`|Dynamic array|May reallocate when growing|
|`String`|UTF-8 string|Allocates heap storage for characters|
|`Rc<T>`|Reference counted|Allocates with counter|
|`Arc<T>`|Atomic reference counted|Thread-safe reference counting|
|`HashMap<K,V>`|Hash map|Allocates buckets and entries|

**Example:**

```rust

fn heap_examples() {
    // Box: single value on heap
    let boxed = Box::new(42);
    println!("{}", *boxed);  // Dereference
    
    // Vec: dynamic array
    let mut vec = Vec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);  // May reallocate here
    
    // String: dynamic string
    let mut s = String::from("Hello");
    s.push_str(" World");  // May reallocate
    
    // Rc: shared ownership
    use std::rc::Rc;
    let shared = Rc::new(vec![1, 2, 3]);
    let clone = Rc::clone(&shared);  // No heap allocation, just increments count
}
```

#### Heap vs Stack Performance

| Operation          | Stack                     | Heap                         |
| ------------------ | ------------------------- | ---------------------------- |
| **Allocation**     | ~1-2 ns (adjust pointer)  | ~50-200 ns (find free block) |
| **Deallocation**   | ~0 ns (just move pointer) | ~50-200 ns (coalesce blocks) |
| **Access**         | ~1-2 ns (direct)          | ~1-2 ns (through pointer)    |
| **Cache locality** | Excellent (sequential)    | Poor (fragmented)            |

---

## Rust's Heap Allocation Methods

### The Allocator API

Rust provides a flexible allocation interface through the `alloc` crate.

#### Basic Allocation Workflow

```rust

use std::alloc::{Layout, alloc, dealloc};
fn manual_allocation() {
    // 1. Define allocation layout (size and alignment)
    let layout = Layout::new::<i32>();
    
    // 2. Allocate memory (unsafe!)
    unsafe {
        let ptr = alloc(layout);
        if ptr.is_null() {
            panic!("Allocation failed");
        }
        
        // 3. Write to memory
        ptr.write(42);
        
        // 4. Read from memory
        let value = ptr.read();
        println!("{}", value);
        
        // 5. Deallocate (must use same layout!)
        dealloc(ptr, layout);
    }
}
```

### Allocation Methods in Rust

#### 1. **Allocator Trait (Unstable)**

```rust

#![feature(allocator_api)]
use std::alloc::Allocator;
fn custom_allocation<A: Allocator>(allocator: A) {
    let layout = Layout::new::<i32>();
    
    // Allocate with custom allocator
    let ptr = allocator.allocate(layout).unwrap();
    
    // Use memory...
    
    // Deallocate
    unsafe { allocator.deallocate(ptr, layout) };
}
```

#### 2. **GlobalAlloc Trait (Stable)**

The primary interface for global allocators (covered in detail below).

#### 3. **Layout for Allocation Requests**

```rust

use std::alloc::Layout;
// For a single type
let layout = Layout::new::<i32>();  // size = 4, align = 4
// For array of types
let layout = Layout::array::<i32>(100).unwrap();  // size = 400, align = 4
// Custom size and alignment
let layout = Layout::from_size_align(1024, 8).unwrap();
// Extend layout
let layout = Layout::new::<i32>();
let (new_layout, offset) = layout.extend(Layout::new::<u64>()).unwrap();
```

#### 4. **Key Allocation Functions**

| Function                         | Purpose                     | When to Use                      |
| -------------------------------- | --------------------------- | -------------------------------- |
| `alloc(layout)`                  | Raw allocation              | Custom allocator implementations |
| `dealloc(ptr, layout)`           | Raw deallocation            | Paired with `alloc`              |
| `realloc(ptr, layout, new_size)` | Resize allocation           | Grow/shrink memory in place      |
| `alloc_zeroed(layout)`           | Zero-initialized allocation | Need zeroed memory               |

---

## The GlobalAlloc Trait

### What Is GlobalAlloc?

`GlobalAlloc` is Rust's interface for the **global memory allocator** - the allocator used by `Box`, `Vec`, `String`, and all standard collection types.

### Trait Definition

```rust

pub unsafe trait GlobalAlloc {
    // Required methods
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);
    
    // Provided methods (with defaults)
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { ... }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 { ... }
}
```

### Why `unsafe` Trait?

Implementing a global allocator requires:

- Handling arbitrary layout requests
    
- Managing memory correctly (no double frees, etc.)
    
- Thread safety (allocators must be sync + send)
    
- Proper alignment guarantees
    

### Default Global Allocator

Rust defaults to the system allocator:

- **Linux/Unix**: `malloc`, `free`, etc.
    
- **Windows**: `HeapAlloc`, `HeapFree`
    
- **WebAssembly**: Custom allocator

```rust

// Default allocator is used automatically
let v = vec![1, 2, 3];  // Uses system malloc
```

### Changing the Global Allocator

#### Method 1: Using `#[global_allocator]` Attribute

```rust

use std::alloc::System;
#[global_allocator]
static GLOBAL_ALLOCATOR: System = System;
fn main() {
    // Now uses System allocator (already default, but explicit)
    let v = vec![1, 2, 3];
}
```

#### Method 2: Custom Allocator Example

```rust

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
struct CountingAllocator {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        
        // Fall back to system allocator
        System.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}
// Must be static and Sync
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator {
    allocations: AtomicUsize::new(0),
    deallocations: AtomicUsize::new(0),
};
fn main() {
    let v = vec![1, 2, 3, 4, 5];
    
    println!("Allocations: {}", ALLOCATOR.allocations.load(Ordering::Relaxed));
    println!("Deallocations: {}", ALLOCATOR.deallocations.load(Ordering::Relaxed));
}
```

### Real-World Allocator Implementations

| Allocator    | Purpose               | Key Features                                   |
| ------------ | --------------------- | ---------------------------------------------- |
| **System**   | Default OS allocator  | General purpose, thread-safe                   |
| **jemalloc** | High performance      | Reduces fragmentation, good for multithreading |
| **mimalloc** | Microsoft's allocator | Very fast, compact                             |
| **snmalloc** | Research allocator    | Message passing friendly                       |
| **TLSF**     | Real-time systems     | O(1) allocation, predictable                   |

### Allocator Requirements and Guarantees

#### Must Provide:

1. **Alignment**: Must satisfy layout's alignment requirement
    
2. **Size**: Must allocate at least layout.size() bytes
    
3. **Thread safety**: Allocator methods may be called from any thread
    
4. **Null on failure**: Return null pointer when allocation fails
    

#### Should Provide:

1. **Good performance**: Reasonable allocation/deallocation speed
    
2. **Low fragmentation**: Reuse freed memory effectively
    
3. **Debug information**: Track allocations for debugging

---
