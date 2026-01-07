# Dependencies

## Required Packages

To build Linux kernel modules, install the following packages:

```bash
sudo apt-get install build-essential linux-headers-$(uname -r)
```

## Package Details

- **build-essential**: Provides core compilation tools (GCC, Make, etc.)
- **linux-headers-$(uname -r)**: Kernel headers matching your running kernel version

## Optional Tools

For advanced development and module signing:

```bash
sudo apt-get install libncurses-dev bison flex libelf-dev libssl-dev
```

- **libncurses-dev**: Required for `make menuconfig`
- **bison** and **flex**: Kernel configuration parsing
- **libelf-dev**: ELF object file handling
- **libssl-dev**: Module signature support

## Module Signing (Secure Boot)

If using Secure Boot:

```bash
sudo apt-get install mokutil
```
