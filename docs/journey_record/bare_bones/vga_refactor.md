
(May 11 2026)

---

This is my Day 5 of `"Writing OS from scratch in Rust` journey - `rusty_os`. 

Today I refactored my `vga_buffer` module.
I updated the `volatile` crate from `0.2` to `0.6.1` version, So I needed to refactor my code.

This crate provides two different wrapper types: [`VolatilePtr`](https://docs.rs/volatile/latest/volatile/struct.VolatilePtr.html "struct volatile::VolatilePtr") and [`VolatileRef`](https://docs.rs/volatile/latest/volatile/struct.VolatileRef.html "struct volatile::VolatileRef"). The difference between the two types is that the former behaves like a raw pointer, while the latter behaves like a Rust reference type. For example, `VolatilePtr` can be freely copied, but not sent across threads because this could introduce mutable aliasing. The `VolatileRef` type, on the other hand, requires exclusive access for mutation, so that sharing it across thread boundaries is safe.

Here's what I refactored:

1. **Simplified Buffer Structure**

	Before: 
	```rust
	struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
	}
	```

	After:

	```rust
	struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
	}
	```

 The `volatile` crate's `VolatilePtr` is a wrapper around a pointer, not the data itself. We want the buffer to store actual `ScreenChar` data, and we'll use `VolatilePtr` to access it with volatile semantics.

2. **Changed Writer's Buffer type**

	Before:
	```rust
	pub struct Writer {
    buffer: &'static mut Buffer,
	}
	```

	After:
	```rust
	pub struct Writer {
    buffer: VolatilePtr<'static, Buffer>,
	}
	```

	`VolatilePtr` provides the volatile read/write operations we need. The `'static` lifetime indicates this pointer is valid for the entire program (the VGA buffer at `0xB8000` is always there). `VolatilePtr` internally uses `NonNull` to guarantee the pointer is never null.

3. **Added Send/Sync Implementations**

	```rust
	unsafe impl Send for Writer {}
	unsafe impl Sync for Writer {}
	```

	The `VolatilePtr` type contains a `NonNull` pointer, which doesn't automatically implement `Send` or `Sync`. However, we're using `Writer`inside a `Mutex` (from the `spin` crate), which requires `Writer` to be `Send`. It's safe here because:

- The VGA buffer at `0xB8000` is a fixed memory address accessible from any CPU core
    
- We always access it through a `Mutex`, ensuring exclusive access
    
- Volatile reads/writes are thread-safe for memory-mapped I/O

4. **Helper Method for clean access**

	```rust
	fn char_ptr(&mut self, row: usize, col: usize) -> VolatilePtr<'_, ScreenChar> {
    unsafe {
        let buffer_ptr = self.buffer.as_raw_ptr().as_ptr();
        let row_ptr = addr_of_mut!((*buffer_ptr).chars[row]);
        let char_ptr = addr_of_mut!((*row_ptr)[col]);
        VolatilePtr::new(NonNull::new_unchecked(char_ptr))
    }
	}
	```

**Why this is necessary:**

- `VolatilePtr` doesn't implement `Index` traits, so we can't do `self.buffer.chars[row][col]`
    
- We need to manually compute pointers to each `ScreenChar`
    
- `addr_of_mut!` creates raw pointers without creating an intermediate reference (important because creating references to MMIO regions can be undefined behavior)
    

**Breaking it down:**

- `self.buffer.as_raw_ptr().as_ptr()` - Gets the raw `*mut Buffer` pointer
    
- `addr_of_mut!((*buffer_ptr).chars[row])` - Gets a pointer to the specific row array
    
- `addr_of_mut!((*row_ptr)[col])` - Gets a pointer to the specific `ScreenChar`
    
- `NonNull::new_unchecked(char_ptr)` - Wraps it in `NonNull` (required by `VolatilePtr`)
    
- `VolatilePtr::new(...)` - Creates the volatile pointer

5. **VolatilePtr in Lazy Static**

	Before:
	```rust
	buffer: unsafe { &mut *(0xB8000 as *mut Buffer) },
	```

	After:
	```rust
	buffer: unsafe { 
    VolatilePtr::new(NonNull::new_unchecked(0xB8000 as *mut Buffer))
	}
	```
	
	The `VolatilePtr::new()` constructor now expects `NonNull<T>` instead of a raw pointer. `NonNull` guarantees the pointer isn't null, providing better safety. We use `NonNull::new_unchecked()` because we know `0xB8000` is a valid non-null address.

6. **Using read() and write() Methods

	Before:
	```rust
	self.buffer.chars[row][col].write(ScreenChar { ... });
	let character = self.buffer.chars[row][col].read();
	```

	After:
	```rust
	self.char_ptr(row, col).write(ScreenChar { ... });
	let character = self.char_ptr(row, col).read();
	```

The API remains clean and simple! `VolatilePtr` provides exactly the `read()` and `write()` methods as I  wanted, maintaining the same ergonomics as my original custom type.

