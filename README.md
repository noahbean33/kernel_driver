# Linux Kernel Character Device Driver

A simple Linux kernel module that implements a character device driver with read/write functionality. This driver creates a device file that allows userspace programs to communicate with kernel space through a 100-byte buffer.

## Overview

This project demonstrates fundamental kernel module development concepts:
- **Character device registration** with a fixed major number (90)
- **File operations** (open, release, read, write)
- **User-kernel space data transfer** using `copy_to_user()` and `copy_from_user()`
- **Kernel buffer management** for storing and retrieving data
- **Module signing** for Secure Boot environments

## Project Structure

```
├── src/                    # Source code
│   └── hello.c            # Main driver implementation
├── docs/                   # Documentation
│   ├── dependencies.md    # Required packages and build tools
│   ├── resources.md       # Useful references and kernel APIs
│   ├── signing_driver.md  # Module signing instructions for Secure Boot
│
├── Makefile               # Build configuration
├── .gitignore             # Git ignore rules
└── README.md              # This file
```

## Prerequisites

Install the required build tools and kernel headers:

```bash
sudo apt-get install build-essential linux-headers-$(uname -r)
```

For Secure Boot systems, you'll also need:
```bash
sudo apt-get install mokutil
```

## Building the Module

```bash
make
```

This will generate `hello.ko` in the project root directory.

## Installation and Usage

### 1. Load the Module

```bash
sudo insmod ./hello.ko
```

Verify it loaded successfully:
```bash
dmesg | tail -n 1
# Output: [ xxxx.xxx] Hello World!
```

### 2. Create the Device File

```bash
sudo mknod /dev/mydev c 90 0
sudo chmod 666 /dev/mydev
```

### 3. Interact with the Device

Write data to the device:
```bash
echo "Hello from userspace!" > /dev/mydev
```

Read data from the device:
```bash
cat /dev/mydev
# Output: Hello from userspace!
```

### 4. Unload the Module

```bash
sudo rmmod hello
```

Check kernel logs:
```bash
dmesg | tail -n 1
# Output: [ xxxx.xxx] Goodbye World!
```

### 5. Cleanup

```bash
sudo rm /dev/mydev
make clean
```

## Module Signing (Secure Boot)

If you're running a system with Secure Boot enabled, you'll need to sign the module. See [`docs/signing_driver.md`](docs/signing_driver.md) for detailed instructions.

Quick steps:
1. Generate a key pair: `openssl req -new -x509 -newkey rsa:2048 -keyout MOK.priv -outform DER -out MOK.der -nodes -days 36500 -subj "/CN=MyCustomKey/"`
2. Sign the module: `sudo /usr/src/linux-headers-$(uname -r)/scripts/sign-file sha256 ./MOK.priv ./MOK.der hello.ko`
3. Enroll the key: `sudo mokutil --import MOK.der` (requires reboot)

## How It Works

The driver creates a kernel buffer that acts as shared storage between userspace and kernel space:

- **Write Operation**: Data from userspace is copied into the kernel buffer (max 99 bytes + null terminator)
- **Read Operation**: Data from the kernel buffer is copied back to userspace
- **Open/Release**: Logs device access events to the kernel log

All operations include proper error handling and bounds checking to prevent buffer overflows and invalid memory access.

## Technical Details

- **Major Number**: 90 (statically assigned)
- **Device Type**: Character device
- **Buffer Size**: 100 bytes
- **Supported Operations**: open, release, read, write
- **Error Codes**: `EFAULT` (copy failures), `EINVAL` (invalid input), `EIO` (registration failure)