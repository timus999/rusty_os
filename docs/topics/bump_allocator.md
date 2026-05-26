A **bump allocator** is an extremely fast memory allocator that manages a contiguous block of memory by maintaining a single "bump pointer." 

Its core principle is simple:

1. It starts with a pointer at the beginning (or end) of a pre-allocated memory region.
    
2. For each allocation, it **returns the current pointer value** and then **increments (or "bumps") the pointer forward** by the size of the requested object. 
    
3. This process allocates memory linearly, one block after another. 
    

The primary advantage is **speed**; an allocation typically requires only a pointer increment and a bounds check, making it significantly faster than general-purpose heap allocators.

---

### Idea

The idea behind a bump allocator is to linearly allocate memory by increasing (_“bumping”_) a `next` variable, which points to the start of the unused memory. At the beginning, `next` is equal to the start address of the heap. On each allocation, `next` is increased by the allocation size so that it always points to the boundary between used and unused memory:
![[bump-allocation.svg]]

The `next` pointer only moves in a single direction and thus never hands out the same memory region twice. When it reaches the end of the heap, no more memory can be allocated, resulting in an out-of-memory error on the next allocation.

### Use Cases in OS

|Use Case|Why Bump Works|
|---|---|
|**Boot-time allocations**|No need to free; just init data structures|
|**Per-system-call arena**|Free entire arena at once|
|**Temporary buffers**|Reset after use|
|**Real-time systems**|O(1), predictable|
|**Single-use allocations**|Never freed (e.g., boot-time config)|

### Advantages and Disadvantages

**Advantages:**

- ✅ Extremely fast (O(1) allocation)
    
- ✅ Simple to implement
    
- ✅ No fragmentation
    
- ✅ Good cache locality (sequential allocations)
    
- ✅ No metadata overhead
    
- ✅ Works without synchronization (single-threaded)
    

**Disadvantages:**

- ❌ No deallocation (or only bulk deallocation)
    
- ❌ Memory leaks if not reset
    
- ❌ Not suitable for general-purpose use
    
- ❌ Cannot reuse freed memory
    
- ❌ Requires knowing heap size in advance