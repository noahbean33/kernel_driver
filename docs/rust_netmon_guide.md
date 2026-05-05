# Linux Driver Development with Rust: Network Monitoring Driver

## Overview

This guide covers building a **network monitoring kernel driver** for Linux in Rust, using FFI to bridge Rust with the C-based Linux kernel. The driver monitors incoming/outgoing packets and supports filtering by protocol (TCP/UDP), IP address (v4), and port.

### References

- [Kernel Rust Quick Start](https://docs.kernel.org/rust/quick-start.html)
- [Rust for Linux Brief Introduction](https://rustmagazine.org/issue-1/rust-for-linux-brief-introduction/)
- [Apriorit: Rust for Linux Driver](https://www.apriorit.com/dev-blog/rust-for-linux-driver)

---

## Benefits of Rust for Linux Driver Development

- **Enhanced memory safety** — Prevents buffer overflows, use-after-free, and null pointer dereferences at compile time via ownership, borrowing, and lifetime rules.
- **Cost-effectiveness** — Early detection of errors (null handling, integer overflows) delivers more stable code out of the box.
- **Performance comparable to C** — Native code generation without a garbage collector; benchmarks show parity with C drivers (e.g. NVMe).
- **Safe concurrency** — Default immutability prevents unintended changes; immutable data can be safely shared across threads without synchronization.
- **Increased developer productivity** — Expressive type system and robust tooling simplify writing, reviewing, and maintaining kernel code.

---

## Challenges

1. **Dual-language kernel maintenance** — FFI bindings between Rust and C create tight coupling; C changes require Rust binding updates.
2. **Linux kernel incompatibility** — Not all kernel features are currently compatible with Rust.
3. **Tooling and compiler instability** — Rust toolchain hasn't reached full stability for all kernel development tasks.
4. **Unstable kernel API** — Frequent API updates and lack of comprehensive documentation increase maintenance burden.

---

## Step 1: Set Up the Environment

1. Download the Linux kernel source code.
2. Install the Rust compiler. (Each Linux version requires a specific Rust version.)
3. Set up `.config` with Rust support — run `make menuconfig` and search for the `RUST` flag:

```
Symbol: RUST [=n]
Type: bool
Defined at init/Kconfig:2001
Prompt: Rust support
Depends on: HAVE_RUST [=y] && RUST_IS_AVAILABLE [=y] && ...
Location:
  (1) -> General setup
       -> Rust support (RUST [=n])
```

4. Enable the appropriate flags in the list, then enable the `RUST` flag.
5. Compile the kernel via LLVM (Clang).
6. Install the kernel on the target test machine.

---

## Step 2: Organize the Project Structure and Build

### Project Layout

```
netmon/
├── Makefile
├── netmon.rs
└── some_source_file.rs
```

### Makefile

```makefile
KDIR ?= /lib/modules/`uname -r`/build
MODULE_NAME := netmon
obj-m := $(MODULE_NAME).o
CC := clang

all:
	make -C $(KDIR) M=$(PWD) modules CC=$(CC)

clean:
	make -C $(KDIR) M=$(PWD) clean
```

---

## Step 3: Use a Foreign Function Interface (FFI)

Since there is no existing Rust API for Linux netfilter, you must implement FFI bindings:

1. Go to the `linux/rust` directory.
2. Create a subdirectory for your API (e.g. `netfilter/`).
3. Create two files:
   - `netfilter_helper.h` — C header including required kernel headers.
   - `lib.rs` — Rust file that generates bindgen bindings.
4. Add the bindings to the kernel build system.

### SkBuff Wrapper

Wrap the kernel's `struct sk_buff` for safe access:

```rust
#[repr(transparent)]
pub(crate) struct SkBuff(UnsafeCell<sk_buff>);
```

- `UnsafeCell` is the core primitive for interior mutability.
- `#[repr(transparent)]` ensures the type has the same representation as `sk_buff`.

### `from_ptr` Function

```rust
impl SkBuff {
    /// Creates a reference to [SkBuff] from a valid pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that ptr is valid and will be valid for the
    /// lifetime of the returned [SkBuff] instance.
    pub(crate) unsafe fn from_ptr<'a>(ptr: *const sk_buff) -> &'a SkBuff {
        // SAFETY: Safety requirements guarantee the validity of the dereference,
        // while the SkBuff type being transparent makes the cast ok.
        unsafe { &*ptr.cast() }
    }
}
```

### Wrapping C Constants with Rust Enums

C defines:
```c
#define NF_DROP 0
#define NF_ACCEPT 1
#define NF_STOLEN 2
#define NF_QUEUE 3
#define NF_REPEAT 4
#define NF_STOP 5
#define NF_MAX_VERDICT NF_STOP
```

Rust equivalent:
```rust
/// Responses from hook functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookResponse {
    /// Drop the packet.
    Drop = netfilter::NF_DROP as _,
    /// Accept the packet.
    Accept = netfilter::NF_ACCEPT as _,
    /// Packet has been "stolen" or consumed by the hook function.
    Stolen = netfilter::NF_STOLEN as _,
    /// Queue the packet to userspace for processing.
    Queue = netfilter::NF_QUEUE as _,
    /// Run the current hook function again.
    Repeat = netfilter::NF_REPEAT as _,
    #[deprecated(note = "Deprecated, for userspace nf_queue compatibility.")]
    Stop = netfilter::NF_STOP as _,
}

impl HookResponse {
    /// The highest possible verdict number.
    #[allow(deprecated)]
    pub(crate) const MAX_VERDICT: HookResponse = HookResponse::Stop;
}
```

---

## Step 4: Initialize and De-initialize the Module

### Module Structure

```rust
/// Structure representing a kernel module.
struct NetMon {
    /// Netfilter hook operations.
    nfho: Pin<Box<NetFilterHookOps>>,
}
```

### Implement `kernel::Module` and `Drop`

```rust
impl kernel::Module for NetMon {
    fn init(_: &'static ThisModule) -> Result<Self> {
        // Create a module instance and register hook operations.
        ...
    }
}

impl Drop for NetMon {
    fn drop(&mut self) {
        // Unregister the hook.
        ...
    }
}
```

### Module Metadata

```rust
module! {
    type: NetMon,
    name: "netmon",
    author: "author",
    description: "Network monitoring module written in Rust",
    license: "GPL",
}
```

---

## Step 5: Test the Code

1. Open diagnostic messages:
```bash
sudo dmesg -wH
```

2. Build and insert the module:
```bash
make
sudo insmod netmon.ko
```

3. Check kernel logs for monitored packet output:
```bash
sudo dmesg
```

Expected output:
```
[Feb18 15:09] netmon: Rust Network Monitor (init)
[  +7.908866] netmon: Tcp: 172.64.41.4:443 -> 192.168.254.135:54964
[  +0.000112] netmon: Packet hex dump:
[  +0.000051] netmon: 000000  00 50 56 2D BB 02 00 50 56 E4 7B 03 08 00 45 00
...
```

The driver monitors packets and records them in Linux kernel logs.
