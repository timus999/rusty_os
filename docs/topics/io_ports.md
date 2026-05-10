The x86 architecture supports two distinct methods for the CPU to communicate with peripheral hardware: **memory-mapped I/O (MMIO)** and **port-mapped I/O (PMIO)**.  These methods differ in how they integrate peripherals into the system's address space and the instructions used to access them.

### 1. Memory-Mapped I/O (MMIO)

In memory-mapped I/O, hardware devices are integrated directly into the system's main memory address space. Specific ranges of memory addresses are reserved and "mapped" to the internal registers of a peripheral device instead of actual RAM. 

- When the CPU reads from or writes to one of these reserved addresses, the request is intercepted by the chipset (e.g., the northbridge) and redirected to the corresponding hardware device.
    
- This allows the use of standard memory access instructions like `MOV`, `LOAD`, and `STORE` for I/O operations.
    
- A classic example is the VGA text buffer, which is mapped to the physical memory address `0xB8000`. Writing data to this address directly updates the text displayed on the screen.
    
- The C `volatile` keyword is often used with MMIO pointers to prevent the compiler from optimizing away seemingly redundant reads or writes, as the hardware can change the value at that address independently. 
    

```
// Example: Writing to the VGA text buffer (MMIO)
volatile char *vga_buffer = (volatile char *)0xB8000;
vga_buffer[0] = 'A'; // This writes the character 'A' to the top-left of the screen
```

### 2. Port-Mapped I/O (PMIO)

Port-mapped I/O, also known as isolated I/O, uses a separate, dedicated address space for I/O devices. This space is distinct from the main memory address space. 

- Communication is performed using special CPU instructions: `IN` to read from a port and `OUT` to write to a port.
    
- Each peripheral is assigned one or more 16-bit port numbers (e.g., the programmable interrupt timer uses ports `0x40` to `0x43`).
    
- Data is transferred between a CPU register and the I/O port. For example, `OUT 0x40, AL` writes the value in the `AL` register to port `0x40`.
    
- This method requires a dedicated I/O bus or a signal (like the `IOR` and `IOW` lines) to differentiate between memory and I/O transactions on the shared address and data buses. 
    

```
; Example: Writing to an I/O port (PMIO)
mov al, 0x3F
out 0x3F8, al ; Write to the COM1 serial port (port 0x3F8)
```
