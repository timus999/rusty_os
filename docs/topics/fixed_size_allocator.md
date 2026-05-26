## Overview and Core Concept

A **fixed size allocator** (also known as a **pool allocator**, **slab allocator**, or **object cache**) is a memory allocation strategy that only handles allocation requests of a single, predetermined size. Unlike general-purpose allocators that must handle arbitrary sizes, a fixed size allocator pre-partitions a memory region into equally sized chunks and manages the allocation and deallocation of these identical blocks.

The fundamental insight behind fixed size allocators is that many real-world systems repeatedly allocate and deallocate objects of the exact same type and size. Process control blocks, file descriptors, network buffers, and page table entries are classic examples of structures that are all identical in size. By dedicating an allocator specifically to this fixed size, we can achieve dramatic improvements in speed, memory overhead, and fragmentation behavior.

## The Core Principles

### Uniform Block Size

The defining characteristic of a fixed size allocator is that every allocation request returns a block of exactly the same size. This size is determined at initialization time and never changes. When a request comes in for memory, the allocator rounds the requested size up to its fixed block size if necessary, or simply fails the request if the requested size exceeds the block size. In practice, fixed size allocators are typically paired with specific structure types, so the size is known precisely.

### Pre-partitioned Memory Pool

The allocator begins by obtaining a large contiguous region of memory from the underlying page allocator. This region is then divided into an array of equally sized slots, each slot capable of holding exactly one object of the fixed size. The number of slots is determined by dividing the total pool size by the fixed block size. This pre-partitioning happens once at initialization and never changes.

### Free List Within the Pool

To track which slots are currently in use, the allocator maintains a free list - a linked list of pointers to available slots. The elegant optimization here is that the free list can be embedded directly within the unused slots themselves. Since a free slot contains no valid data, its first few bytes can be repurposed to store a pointer to the next free slot. This means the allocator requires no additional memory for metadata beyond what it already manages.

## Operational Behavior

### Allocation Operation

When an allocation request arrives, the allocator simply removes the first slot from the free list and returns a pointer to it. This operation involves just a few pointer manipulations and does not depend on how many slots are currently allocated or free. There is no searching, no splitting of blocks, no coalescing of adjacent free space. The time complexity is constant, O(1), and extremely predictable.

If the free list is empty, meaning all slots are currently allocated, the allocator can respond in several ways. It might return a null pointer to indicate allocation failure. It might automatically request an additional page from the page allocator, expand the pool by adding more slots, and then satisfy the request from the new slots. Or it might wait in a queue until a slot becomes available.

### Deallocation Operation

Deallocation is equally simple. When an object is freed, the allocator takes the pointer being returned, treats the memory it points to as a free list node, and inserts it back at the head of the free list. No coalescing is needed because all slots are the same size and adjacent slots have no special relationship. The deallocated slot becomes immediately available for the next allocation. Like allocation, deallocation is O(1) with minimal overhead.

## Memory Layout and Organization

### Linear Array Model

In its simplest form, a fixed size allocator lays out slots contiguously in memory. The first few bytes of each slot serve as the next pointer when the slot is free. When a slot is allocated, those same bytes belong to the user's object and can be overwritten with any data. This dual use of the slot's memory is what eliminates the need for separate metadata structures.

### Per-Slot Overhead

The only overhead per slot is the storage needed for the free list pointer, which is typically the size of a pointer (4 or 8 bytes). However, this pointer only exists when the slot is free. When the slot is allocated, that memory is fully available to the user. Some implementations add additional debugging metadata, such as magic numbers to detect corruption or allocation tracking information, but these are optional and usually disabled in production kernels.

### Cache Friendliness

Fixed size allocators exhibit excellent cache behavior because objects of the same type tend to be allocated close together in memory. When the allocator returns a slot, that slot resides in the same contiguous memory region as all other slots of its type. This spatial locality means that iterating over arrays of these objects or accessing related objects will likely hit cache lines that are already loaded.

## Variants and Specializations

### Power-of-Two Fixed Size

Some fixed size allocators restrict block sizes to powers of two, such as 16, 32, 64, 128 bytes, and so on. This simplifies address calculations because finding a slot's index requires only shifting instead of division. It also ensures natural alignment for any data type that fits within the block.

### Multiple Fixed Size Classes

To handle objects of different sizes, a system might maintain multiple fixed size allocators, each for a different size class. The allocator for 32-byte objects handles requests for structures up to 32 bytes, the 64-byte allocator handles requests between 33 and 64 bytes, and so on. The general-purpose allocation function examines the requested size, selects the appropriate size class, and forwards the request to that class's allocator. This is the foundation of slab allocation and similar techniques.

### Per-CPU Caches

In multiprocessor systems, a single free list protected by a lock becomes a contention point. To address this, the allocator can maintain per-CPU free lists. Each CPU has its own cache of free slots that it can access without locking. When a CPU's cache is exhausted, it replenishes from a global pool with a single lock operation. When a CPU frees an object, it returns it to its local cache. This dramatically reduces lock contention and improves scalability.

### Colored Allocators

A problem with fixed size allocators is that different objects might end up on the same cache line, causing false sharing. In a false sharing scenario, two CPUs each own a different object that happens to reside on the same cache line. When each CPU modifies its own object, the cache line bounces between CPUs, degrading performance. A colored allocator addresses this by adding small offsets to the starting addresses of different pools, spreading the objects across cache lines.

## Advantages Over General Allocators

### Predictable Performance

The most significant advantage of fixed size allocators is their predictable, constant-time performance. Every allocation and deallocation takes the same small number of CPU cycles regardless of how many objects are allocated or how fragmented the heap might be. This predictability is crucial for real-time systems, interrupt handlers, and other time-sensitive kernel code.

### No Internal Fragmentation

Because all slots are exactly the size needed for the objects they store, there is no internal fragmentation. Internal fragmentation occurs when an allocator must round up a request to some minimum size or alignment, wasting space within the allocated block. Fixed size allocators eliminate this waste entirely by matching the block size precisely to the object size.

### Minimal External Fragmentation

External fragmentation occurs when free memory is broken into small pieces that cannot satisfy larger allocations. Fixed size allocators operating on a single size class never suffer from this problem because all free slots are identical and interchangeable. No matter how allocation and deallocation are interleaved, any free slot can satisfy any allocation request.

### Low Metadata Overhead

General-purpose allocators often require metadata per allocation, such as block size, next/previous pointers, and consistency checks. This metadata consumes memory even when the block is allocated. Fixed size allocators embed the free list pointer only in free blocks, so allocated blocks have zero metadata overhead within the pool itself.

## Limitations and Disadvantages

### Memory Waste for Varying Sizes

If the fixed size allocator is used for objects that sometimes require less than the full block size, the unused portion of each slot becomes wasted. This internal fragmentation can be significant if the block size is large but many allocations are small. This is why systems typically maintain multiple size classes rather than a single fixed size.

### Pool Size Management

Determining the right pool size is challenging. If the pool is too small, the allocator may exhaust its slots and need to grow, which requires obtaining additional pages and integrating them into the free list. If the pool is too large, memory is wasted on slots that are never used. Some systems address this by starting with a small pool and expanding dynamically as demand increases.

### Rigid Size Handling

A fixed size allocator cannot handle an allocation request that exceeds its fixed block size, even if the system has plenty of free memory elsewhere. This means the allocator must be paired with some mechanism for handling oversize requests, typically by falling back to a general-purpose allocator or using a larger size class.

### Debugging Challenges

When a fixed size allocator reuses memory immediately after deallocation, it becomes possible for dangling pointers (pointers to freed memory) to cause subtle bugs. The allocator may return the same memory address for a new allocation, and code that still holds the old pointer might inadvertently corrupt the new object. Some debugging implementations add delay to reclamation or keep freed objects separate to catch such bugs.

## Relationship to Page Allocators

Fixed size allocators operate at a higher level than page allocators. While a page allocator deals with entire pages (typically 4KB or larger), a fixed size allocator subdivides pages into smaller chunks. The fixed size allocator requests pages from the page allocator when it needs to expand its pool and returns pages when a pool becomes completely empty and is unlikely to be needed again.

This relationship creates a two-level hierarchy. The page allocator provides coarse-grained physical or virtual memory in page-sized units. The fixed size allocator provides fine-grained allocations for specific object types. This hierarchy works well because the page allocator has its own fragmentation concerns that are mitigated by the fixed size allocator's disciplined use of whole pages.

## Integration with Virtual Memory

In kernels that use virtual memory, fixed size allocators have additional flexibility. When a fixed size allocator needs to expand its pool, it can request new virtual pages from the virtual memory manager and map them into the allocator's address space. The allocator can then carve these new pages into additional slots and add them to the free list.

The use of virtual memory also enables protection features. A debug kernel might map guard pages between slots or at the boundaries of the pool to catch out-of-bounds accesses. When a slot is freed, the kernel could even unmap its page temporarily to catch use-after-free bugs, though this carries a significant performance penalty.

## Real-World Examples in Operating Systems

### Linux Slab Allocator

The Linux kernel uses slab allocation extensively for frequently allocated structures. Each slab cache is dedicated to a specific structure type, such as task structures, file objects, or socket buffers. The slab allocator maintains per-CPU caches for hot objects and uses coloring to improve cache behavior. It also includes constructors and destructors that initialize and clean up objects when they enter and leave the cache.

### FreeBSD UMA

FreeBSD's Unified Memory Allocator (UMA) is another sophisticated fixed size allocator. It supports zones of fixed-size objects, per-CPU caches, and batch allocation and deallocation. UMA can also create secondary zones that allocate objects from a primary zone but add additional features like zero-initialization.

### Windows Lookaside Lists

Windows uses lookaside lists for fixed-size allocations. A lookaside list is essentially a fixed size allocator with per-CPU caches. Executive lookaside lists are used for system structures, while custom lookaside lists can be created by drivers for their own frequently allocated structures.

## When to Use Fixed Size Allocators

Fixed size allocators are ideal when the following conditions hold:

The system repeatedly allocates and deallocates many objects of identical size. The allocation and deallocation patterns are unpredictable, but the object size is constant. Performance and predictability are more important than minimizing total memory usage. The number of simultaneously active objects has a reasonable upper bound that can be estimated.

They are less suitable when objects vary significantly in size, when the total number of objects is unknown and potentially enormous, when memory is extremely constrained and every byte counts, or when objects have complex initialization requirements that differ from the simple allocation pattern.

## Theoretical Performance Analysis

From a theoretical perspective, fixed size allocators achieve the optimal possible performance for dynamic memory allocation. The allocation operation requires a constant number of memory accesses: one to read the free list head from a global variable, and one to update that variable to point to the next free slot. Deallocation similarly requires just two memory accesses: one to write the new free list head into the slot being freed, and one to update the global head pointer.

This is asymptotically optimal because any allocation must at minimum return a unique pointer, and any deallocation must at minimum make that pointer available again. The fixed size allocator achieves this with no additional overhead, no searching, and no complex data structure maintenance.

## Comparison With Other Allocator Types

Compared to bump allocators, fixed size allocators support individual deallocation but have higher per-slot overhead and are limited to a single size. Compared to linked list allocators, fixed size allocators are dramatically faster and have no external fragmentation but cannot handle variable sizes efficiently. Compared to buddy allocators, fixed size allocators are simpler and faster for their specific size but lack the buddy allocator's ability to handle multiple sizes with limited fragmentation.

The fixed size allocator occupies an important niche in the allocator ecosystem. It is not a general-purpose solution, but for its intended use case - allocating many identical objects - it is unmatched in both performance and memory efficiency. This is why virtually every production operating system kernel uses fixed size allocators extensively for its core data structures.