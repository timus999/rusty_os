A **linked list allocator** is a type of heap allocator that manages free memory by using a **linked list of free blocks**, where each block contains a pointer to the next free block. 

Its operation is as follows:

1. **Free List**: It maintains a list (the "free list") that tracks all currently unallocated memory regions. This list can be ordered by address or kept in a simple LIFO (last-in, first-out) order. 
    
2. **Allocation**: When a memory request is made, the allocator **traverses the free list** to find a block large enough to satisfy the request.  Common strategies include:
    
    - **First-fit**: Uses the first suitable block found.
        
    - **Best-fit**: Searches the entire list for the smallest block that fits, minimizing waste. 
        
3. **Deallocation**: When memory is freed, the block is **added back to the free list**.  A critical optimization is **coalescing**, where the allocator checks if the freed block is adjacent to other free blocks in memory and **merges them into a single larger block** to reduce fragmentation. 
    

This design allows for flexible allocation and deallocation in any order but can be slower than simpler allocators due to the need to search the list and manage block merging.

![[linked-list-allocation.svg]]

Each list node contains two fields: the size of the memory region and a pointer to the next unused memory region. With this approach, we only need a pointer to the first unused region (called `head`) to keep track of all unused regions, regardless of their number. The resulting data structure is often called a [_free list_](https://en.wikipedia.org/wiki/Free_list).

### Drawbacks:

- **Slow Allocation Speed**: Unlike bump allocators, allocation is **not constant time**.  The allocator must **traverse the free list** to find a block large enough for the request.  In a fragmented heap with many small free blocks, this search can become significantly slow, potentially requiring a scan of the entire list.
    
- **External Fragmentation**: Repeated allocation and deallocation of varying sizes can scatter free memory into many small, non-contiguous holes. Even if the **total free memory is sufficient**, the allocator may fail to satisfy a request for a large block if no single hole is big enough.  While **coalescing** (merging adjacent free blocks) mitigates this, it adds computational overhead to the deallocation process. 
    
- **Memory Overhead**: Each free block must store **metadata** (such as its size and a pointer to the next free block) within the block itself.  This reduces the total amount of memory available for actual program data, imposing a minimum size limit on allocations (typically at least the size of two pointers).
    
- **Poor Cache Locality**: Because free blocks can be scattered anywhere in the heap, traversing the linked list often results in **cache misses**, further degrading performance compared to allocators that keep memory contiguous.
