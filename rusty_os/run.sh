#!/usr/bin/bash

qemu-system-x86_64 -drive format=raw,file=target/x86_64-rusty_os/debug/bootimage-rusty_os.bin -s -S

# qemu-system-x86_64 \
#     -drive format=raw,file=target/x86_64-rusty_os/debug/bootimage-rusty_os.bin
#     -serial stdio \
#     -s -S \
#     -cpu qemu64 \
#     -m 512M \
#     -no-reboot \
#     -d guest_errors \
#     -machine q35
