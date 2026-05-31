
(May 9 2026)

--- 

This is Day 3 of my "writing OS from scratch in rust" - `rust_os`.

Today I dove deep in to [`VGA`](topics/vga_text_mode), I then understood how text and graphics are displayed on the screen. 

Then I followed the `VGA Text Mode` post of Philipp Oppermann's blog.

--- 

I created rust module `vga_buffer` which handles printing on the screen.

### Colors

Then I created colors enum which represent different colors supported by `VGA text mode`. 

Here I learned about the [`repr`](rust_topics/records/repr), what it is and why it is used.

Then I created `ColorCode` struct which holds the foreground and background color.

Here I learned about `Rust` and `C` struct type, their memory layout, optimization done by `Rust` compiler and difference between them.

---
### Text Buffer

I created `ScreenChar` struct which represent character and the color code to be printed on the screen.

Then I defined two const which is screen height and width (25, 80) as learned `VGA`.

I also created `Buffer` array for reading and writing.

Then I created `Writer` struct which is responsible for writing character on the buffer.

---

### Printing

I implemented `Writer` struct with some function that helps to write on the `Buffer`.

Then I tested out by writing something and it worked.


![[Screenshot From 2026-05-09 16-29-57.png]]

---

### Volatile

This part I understand a little bit but I need to dive deep tomorrow. Basically `Rust` tries to optimize and some side effects can be seen which we don't want.
So we make sure the optimization doesn't happen.

I done this by adding `volatile` crate and wrapping `Buffer` with `volatile`.

---

### Formatting macros

This one is make our life easier.
I just added `Rust's formatting macros` support by implementing `core::fmt::Writ` trait for `Writer`.

---

### A Global Interface

I needed to provide a global writer that can be used as an interface from other modules without carrying a `Writer` instance around.

But If tried to create `static Writer`, the compiler would give error.

I knew there was crate name `lazy_static` which helps in one-time initialization of statics with non-const functions.

So I added `lazy_static` crate with `spin_no_std` feature since I don't link the standard library.

---
### Spinlocks

However there is a problem. The `Writer` is pretty useless since it is immutable. And there is some ways to make `mutable` don't that will cause other problem.
I will learn more about this tomorrow.

So there is a crate named `spin` which helps to add `safe interior mutability` to static `WRITER`.

I wrapped the `WRITER` with `spin::Mutex`.

---

### A print macro

I basically modified  `print` and `println` macro to use our own `_print` function.

So that I can use `print` and `println` macro anywhere.

I need to study the macro structure which I'm confused tomorrow.

---

### Hello World using println

After that, I printed "Hello world!" using `println!` macro.

![[Screenshot From 2026-05-09 16-46-30.png]]

Then I also used `println` macro to print panic message.

![[Screenshot From 2026-05-09 16-48-16.png]]

---

This much for today. I learned so many things today.