
The `#[repr(...)]` attribute in Rust explicitly controls the **memory layout** and **ABI representation** of composite types (structs, enums, and unions).  By default, types use `#[repr(Rust)]`, which allows the compiler to optimize layout freely (reordering fields, adding padding) for performance, but this layout is **not guaranteed** to remain consistent across compiler versions or crates. 

### Common Repr Attributes

- **`#[repr(C)]`**: Enforces a memory layout identical to the **C programming language**.  Fields are laid out in declaration order with standard padding. This is essential for **FFI** (Foreign Function Interface) to ensure Rust structs are compatible with C libraries or hardware registers. 
    
- **`#[repr(transparent)]`**: Guarantees that a newtype wrapper (a struct with a single non-ZST field) has the **exact same representation** as its inner field, including ABI calling conventions.  This is critical for FFI when wrapping primitive types (e.g., `struct MyInt(i32)`) to ensure they are passed identically to the underlying type.
    
- **`#[repr(packed)]`**: Eliminates all **padding** between fields to minimize memory usage.  This is useful for network protocols or specific hardware formats but can cause **performance penalties** or **undefined behavior** if fields are accessed as unaligned references on certain architectures. 
    
- **`#[repr(align(N))]`**: Forces the type to have a specific **alignment** (a power of two), which is useful for interacting with hardware that requires specific memory alignment or for SIMD operations.

---

### Key Differences

| Attribute        | Layout Guarantee          | Primary Use Case            | ABI Compatibility       |
| ---------------- | ------------------------- | --------------------------- | ----------------------- |
| `Rust` (Default) | None (Compiler optimized) | General safe Rust code      | Rust-only               |
| `C`              | Predictable, C-like       | FFI, Interoperability       | C-compatible            |
| `Transparent`    | Identical to single field | Newtype wrappers for FFI    | Identical to inner type |
| `Packed`         | Tight, no padding         | Memory-constrained/hardware | Unaligned access risks  |
| `Align(N)`       | Custom alignment          | SIMD, Hardware specs        | N/A (Alignment only)    |