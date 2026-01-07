Advanced Engineering Principles in Linux Kernel Driver Development: Architectures, Mechanisms, and Modern Paradigms
Executive Summary
The discipline of Linux kernel driver development sits at the intersection of hardware interfacing, operating system architecture, and high-performance concurrency management. Unlike user-space application development, where the operating system provides a layer of abstraction and protection, kernel-space programming operates within the privileged CPU Ring 0 environment. Here, a single logical error does not merely crash an application; it compromises the stability, security, and integrity of the entire system. This comprehensive report analyzes the technical foundations, architectural evolution, and modern best practices for developing Linux kernel drivers as of the mid-2020s.
We examine the complete lifecycle of a kernel module, from the intricacies of the kbuild system to the dynamic registration of character devices. A significant portion of this analysis is dedicated to the evolution of kernel APIs, specifically the transition from coarse-grained locking mechanisms (such as the Big Kernel Lock) to fine-grained concurrency primitives like unlocked_ioctl and atomic operations. Furthermore, we explore the paradigm shift introduced by Managed Device Resources (devres), which fundamentally alters how drivers manage memory lifecycles to prevent leaks.
Crucially, this report addresses the emerging role of the Rust programming language in the Linux kernel—a historic shift aimed at introducing compile-time memory safety to a codebase dominated by C for over three decades. Through a rigorous examination of memory management strategies, including the secure transfer of data across the user-kernel boundary and the pitfalls of devm_kzalloc usage, this document serves as a definitive technical reference for systems engineers. The analysis synthesizes data from technical documentation, kernel source trees, and community discussions to provide a nuanced understanding of the current state of the art in Linux driver engineering.
1. The Kernel Execution Environment and Architecture
To understand the engineering constraints of a device driver, one must first comprehend the execution environment of the Linux kernel. The operating system employs a monolithic architecture, meaning the entire operating system kernel—including file systems, networking stacks, and device drivers—runs in a single, shared address space with high privileges.
1.1 Privilege Separation: Ring 0 vs. Ring 3
The fundamental security mechanism of the x86 architecture (and similar protections in ARM and RISC-V) is the concept of protection rings. User-space applications, such as web browsers and shells, execute in Ring 3. In this mode, the CPU restricts access to critical instructions and hardware resources. An application cannot directly access the network card or physical RAM; it must request these services from the kernel via system calls (syscalls).1
When a syscall is invoked, the CPU triggers a context switch, transitioning execution to Ring 0—the kernel mode. Device drivers operate exclusively in this privileged mode. This grants them unrestricted access to the machine's hardware and memory but imposes severe responsibilities. The kernel stack is extremely small (typically 8KB to 16KB) compared to the user-space stack (often 8MB+). Consequently, drivers must avoid deep recursion and large stack-allocated structures. Large data structures must be allocated dynamically on the heap using specialized kernel allocators.1
1.2 Process Address Space and Kernel Split
On a typical 32-bit architecture, the 4GB virtual address space is split between the user process and the kernel. The standard configuration allocates the lower 3GB to the user space and the upper 1GB to the kernel (the 3G/1G split). While user-space memory is swapped out and virtually addressed, the kernel logical address space is permanently mapped. On 64-bit systems, the address space is vastly larger, but the principle of separation remains.1
This separation dictates how data is exchanged. A pointer generated in user space is a virtual address valid only within the context of that specific process. The kernel cannot dereference this pointer directly because it may point to unmapped memory, swapped-out pages, or invalid addresses intended to trigger a fault. This necessitates the use of specialized data transfer routines like copy_from_user and copy_to_user, which handle the translation and safety checks required to move data across this boundary.2
1.3 The Concept of Reentrancy
Kernel code must be reentrant. In a symmetric multiprocessing (SMP) environment, the same driver code can be executed simultaneously on multiple CPUs. Furthermore, a driver function might be suspended (put to sleep) to wait for an I/O event, and while it sleeps, another process might invoke the same driver function. This concurrency requires that drivers be designed without reliance on global state unless that state is explicitly protected by synchronization primitives like mutexes or spinlocks. The failure to ensure reentrancy leads to race conditions—transient bugs that are notoriously difficult to reproduce and debug.1
2. Development Environment and Build Infrastructure
Establishing a robust development environment is the prerequisite for kernel engineering. Unlike standard C development, which relies on libc and standard headers, kernel development requires a specific set of kernel headers and build tools that match the target kernel version exactly.
2.1 Distribution-Specific Prerequisites
The specific packages required to build kernel modules vary by Linux distribution, though the underlying tools remain consistent.
Debian/Ubuntu Ecosystem:
On Debian-based systems, the meta-package build-essential provides the core compilation tools (GCC, Make). However, the specific headers for the running kernel must be installed separately. The package linux-headers-$(uname -r) ensures that the headers match the currently running kernel version. Additional tools are often required for configuration and signing:
libncurses-dev: Required for make menuconfig, the terminal-based kernel configuration interface.
bison and flex: Essential for parsing the kernel's configuration logic.
libelf-dev and libssl-dev: Used for building module signatures and handling ELF object files.4
Red Hat/Fedora Ecosystem:
Fedora and RHEL use a different package naming convention. The core headers are found in kernel-headers and kernel-devel. The build environment is set up using dnf install @development-tools. Fedora also emphasizes the use of specific branches for development (e.g., git switch f38 for Fedora 38 kernels) when working directly with the kernel source tree.5
Rust Toolchain Requirements:
With the introduction of Rust support in Linux 6.1, the toolchain requirements have expanded. Developers wishing to write Rust drivers need rustc, the Rust compiler, and bindgen, a tool that automatically generates Rust FFI (Foreign Function Interface) bindings to C header files. Distributions like Arch Linux and Fedora provide these via packages like rust-src and bindgen-cli, while others may require installing specific versions of LLVM/Clang to match the kernel's requirements.7
2.2 The Kbuild System
The Linux kernel uses a specialized build system called Kbuild. A standard Makefile is not sufficient for building a kernel module because the module must be linked against the kernel's symbol table and compiled with specific flags defined by the kernel configuration.
The standard pattern for an external module's Makefile utilizes a "two-pass" build strategy.
The Standard Makefile Template:

Makefile


# If KERNELRELEASE is defined, we've been invoked from the
# kernel build system and can use its language.
ifneq ($(KERNELRELEASE),)
    obj-m := mydriver.o
    mydriver-y := main.o utils.o

# Otherwise we were called directly from the command
# line; invoke the kernel build system.
else
    KDIR?= /lib/modules/$(shell uname -r)/build
    PWD := $(shell pwd)

default:
    $(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
    $(MAKE) -C $(KDIR) M=$(PWD) clean
endif


Mechanism of Action:
Pass 1: When the user types make, the KERNELRELEASE variable is not set. The else block executes, setting the KDIR variable to the kernel build directory (usually a symlink to the headers). It then executes $(MAKE) -C $(KDIR) M=$(PWD) modules. The -C flag changes the directory to the kernel source tree, effectively handing control over to the kernel's top-level Makefile.8
Pass 2: The kernel's Makefile sets up the environment (defining KERNELRELEASE) and calls the module's Makefile again. This time, the ifneq block executes, and Kbuild reads the obj-m variable to determine which object files to build.8
New in Linux 6.13:
Starting with kernel version 6.13, a streamlined syntax allows external modules to include the kernel Makefile directly, reducing the complexity of the recursive make calls. This indicates an ongoing effort to simplify the developer experience for out-of-tree modules.8
2.3 Kernel Configuration and Versioning
The build process is heavily influenced by the kernel's configuration file (.config), typically located in /boot/config-$(uname -r) or the kernel source root. This file defines which features are enabled (e.g., CONFIG_SMP, CONFIG_PREEMPT). A driver built for a kernel with Symmetric Multiprocessing (SMP) enabled will not load on a non-SMP kernel due to layout mismatches in concurrency structures.6
The kernel version is also encoded in the vermagic string within the module. When insmod attempts to load a module, it compares the module's vermagic with the running kernel's version. If they differ, the load is rejected to prevent system instability. This underscores the necessity of rebuilding drivers for every specific kernel update.11
3. The Anatomy of a Loadable Kernel Module (LKM)
A kernel module is not an autonomous executable; it is an object file linked dynamically into the running kernel. Its structure is defined by specific entry and exit points and metadata macros.
3.1 Entry and Exit Points
Unlike C programs that begin execution at main(), kernel modules utilize the module_init() and module_exit() macros to designate their constructor and destructor functions.
Initialization (module_init): This function is called when the module is loaded. It is responsible for registering the device, allocating memory, and requesting hardware resources (IRQs, DMA channels). It returns 0 on success or a negative error code (e.g., -ENOMEM, -EBUSY) on failure. If initialization fails, the module is not loaded.12
Cleanup (module_exit): This function is called when the module is unloaded. It is critical that this function releases all resources allocated during initialization. Failing to free memory or unregister a device will leave the kernel in an inconsistent state, often requiring a reboot to fix.12
The __init and __exit Macros:
Functions marked with __init (e.g., static int __init my_init(void)) are placed in a special ELF section (.init.text). Once initialization is complete, the kernel frees this memory, reclaiming it for other uses. Similarly, __exit functions can be discarded if the kernel is compiled without module unloading support, reducing the kernel footprint.13
3.2 Module Metadata and Licensing
The kernel uses macros to embed metadata into the module's binary.
MODULE_LICENSE("GPL"): This is arguably the most critical macro. It declares the license of the code. If a module is not marked as GPL-compatible, the kernel is "tainted" when the module loads. Tainting disables certain debugging features and warnings, and kernel developers will generally refuse to debug issues on a tainted kernel. Furthermore, non-GPL modules cannot access symbols exported with EXPORT_SYMBOL_GPL, severely limiting their functionality.1
MODULE_AUTHOR, MODULE_DESCRIPTION, MODULE_VERSION: These provide informational data visible via the modinfo command.11
3.3 Symbol Exporting
Modules can export functions to be used by other modules using EXPORT_SYMBOL(func) or EXPORT_SYMBOL_GPL(func). This mechanism allows for the creation of stacked drivers, where a core module handles low-level hardware access and allows other modules to register as clients. This is common in subsystems like USB or subsystems where a "bus" driver exposes an API to "device" drivers.10
4. The Character Device Subsystem
Character devices (cdev) represent one of the three main classes of Linux devices (alongside block and network devices). They are abstractions for hardware that can be accessed as a stream of bytes, such as serial ports, sensors, or sound cards.
4.1 Device Identification: Major and Minor Numbers
Internally, the kernel identifies character devices using a pair of numbers encoded in the dev_t type:
Major Number: Identifies the associated driver. All devices managed by the same driver share the same major number.
Minor Number: Identifies the specific device instance. For example, /dev/ttyS0 and /dev/ttyS1 share the major number for the serial driver but have different minor numbers.16
Dynamic Allocation:
Historically, major numbers were statically assigned. However, given the finite number of available slots, modern drivers utilize dynamic allocation. The function alloc_chrdev_region() asks the kernel to reserve a range of numbers dynamically.
Prototype: int alloc_chrdev_region(dev_t *dev, unsigned int firstminor, unsigned int count, char *name);
Usage: This ensures the driver does not conflict with existing devices. The macros MAJOR(dev) and MINOR(dev) are used to extract the components from the returned dev_t.16
4.2 The cdev Structure and Registration
The struct cdev is the kernel's internal representation of a character device. The registration process involves initializing this structure and linking it to the file operations.
Initialization Sequence:
Allocation: Define a struct cdev.
Initialization: Call cdev_init(&my_cdev, &fops). This associates the device with the defined file operations.
Registration: Call cdev_add(&my_cdev, dev_no, 1).
Critical Warning: The moment cdev_add is called, the device becomes "live." The kernel may immediately begin routing system calls to the driver's functions. Therefore, cdev_add must be the last step in the initialization process, performed only after all memory is allocated, locks are initialized, and hardware is ready.16
4.3 File Operations (struct file_operations)
The file_operations structure (commonly fops) is the interface between the Virtual File System (VFS) and the driver. It consists of function pointers that implement the standard system calls.
Table 1: Key File Operations
Operation
Driver Function Pointer
Description
Open
.open
Called when the user opens the device file. Used to allocate private data or increment usage counts.
Read
.read
Transfers data from the device to user space. Must handle partial reads.
Write
.write
Transfers data from user space to the device.
IO Control
.unlocked_ioctl
Handles device-specific commands (e.g., "reset device", "set baud rate").
Memory Map
.mmap
Maps device memory directly into user space for high-performance access.
Release
.release
Called when the file descriptor is closed. Used to free private data.

If a driver sets a pointer to NULL, the kernel provides a default behavior (usually returning an error or success if the operation is trivial, like open).19
5. The Linux Device Model and Sysfs Integration
Creating a character device internally (cdev_add) makes it usable by the kernel, but it does not automatically create a file in the /dev directory for the user. In the past, users had to manually run mknod. Today, the Linux Device Model automates this via udev.
5.1 Classes and Device Creation
The integration works by exporting device information to sysfs, a virtual filesystem usually mounted at /sys. The udev daemon monitors /sys for changes and automatically creates or removes nodes in /dev.
The class_create Mechanism:
Drivers first register a "class" of devices.
struct class *cls = class_create("my_class_name");
This creates a directory /sys/class/my_class_name.
API Instability Note (Linux 6.4+):
The class_create function signature is a prime example of the kernel's fluid API.
Before Kernel 6.4: class_create(THIS_MODULE, "name")
Kernel 6.4 and later: class_create("name")
The owner argument (pointer to struct module) was removed. Drivers intended to support a wide range of kernel versions must use preprocessor conditionals (#if LINUX_VERSION_CODE >=...) to handle this, as noted in recent build failure reports.21
5.2 Device Registration
Once the class exists, the driver registers individual devices belonging to that class using device_create.
Prototype: struct device *device_create(struct class *cls, struct device *parent, dev_t devt, void *drvdata, const char *fmt,...);
This creates the necessary sysfs entries. The drvdata argument allows the driver to attach a pointer to private data, which can later be retrieved in callbacks using dev_get_drvdata. While NULL is often passed if no private data is needed, passing a valid pointer is useful for object-oriented driver designs where multiple device instances share the same driver code.23
Upon successful execution, udev receives a "uevent," reads the major/minor number and device name from sysfs, and creates the /dev/my_device node with the appropriate permissions.26
6. Memory Management Paradigms
Memory management in the kernel is fundamentally different from user space. There is no automatic garbage collection, and memory is strictly divided into different zones and types.
6.1 Kernel Allocators: kmalloc vs. vmalloc
The workhorse of kernel memory allocation is kmalloc.
Characteristics: It allocates memory that is physically contiguous. This is crucial for Direct Memory Access (DMA), as hardware controllers typically do not understand virtual addresses and require contiguous physical blocks to read/write data.
Flags: The behavior of kmalloc is controlled by flags. GFP_KERNEL is the standard flag, which allows the allocator to sleep (block) while waiting for memory to become available. This means kmalloc(..., GFP_KERNEL) cannot be called from interrupt context or while holding a spinlock. For atomic contexts, GFP_ATOMIC must be used, which fails immediately if memory is not free, rather than sleeping.27
For large allocations where physical contiguity is not required (e.g., large internal software buffers), vmalloc is used. It stitches together non-contiguous physical pages into a contiguous virtual address range. While vmalloc is easier on the allocator, it incurs a performance penalty due to Translation Lookaside Buffer (TLB) thrashing and cannot be used for DMA.27
6.2 Safe Data Exchange with User Space
A major security boundary exists between the kernel and user space. A naive driver might attempt to dereference a user-provided pointer directly:

C


// DANGEROUS - DO NOT DO THIS
char val = *user_ptr;


This is catastrophic. The pointer might refer to unmapped memory (causing a kernel panic), or worse, a malicious user might pass a pointer to a kernel data structure, tricking the driver into reading or overwriting sensitive kernel memory.
The copy_ Functions:
To safely transfer data, the kernel provides copy_from_user and copy_to_user.
Mechanism: These functions first check if the pointer is within the valid user-space range for the calling process (using access_ok). They then perform the copy while handling potential page faults.
Return Value: They return 0 on success. If the copy fails (e.g., the user buffer was invalid), they return the number of bytes failed to copy.
Security Feature: copy_from_user has a built-in safety feature: if the copy fails partway through, it zeroes the remainder of the kernel buffer. This prevents the kernel from accidentally processing uninitialized stack garbage as valid user data.2
Why not memcpy?
Using memcpy is insecure because it does not perform access checks and does not handle page faults. If a memcpy encounters a swapped-out page in user space, it will cause a kernel oops. copy_from_user knows how to handle this by transparently faulting the page in.2
7. Concurrency and Synchronization
Linux is a preemptive, symmetric multiprocessing (SMP) operating system. This means that a driver function can be interrupted at any instruction, and multiple instances of the same function can run simultaneously on different CPUs. Without protection, this leads to race conditions and data corruption.
7.1 Spinlocks: The Busy-Wait Primitive
Spinlocks (spinlock_t) are the simplest locking primitive. When a thread attempts to acquire a locked spinlock, it enters a tight loop ("spins"), constantly checking if the lock has become free.
Context: Spinlocks are designed for code that cannot sleep, such as interrupt handlers.
Constraint: You cannot sleep while holding a spinlock. Calling kmalloc(GFP_KERNEL), copy_to_user, or mutex_lock while holding a spinlock will lead to a system deadlock or crash. The CPU is essentially "frozen" for other tasks while the lock is held, so critical sections must be extremely short (a few lines of code).28
7.2 Mutexes: The Sleeping Lock
Mutexes (struct mutex) allow a thread to sleep while waiting for the lock. If the lock is held by another thread, the scheduler puts the requesting thread to sleep and switches to another task.
Context: Mutexes are used in process context (e.g., system calls like read, write, ioctl) where sleeping is permitted.
Usage: They are suitable for protecting large critical sections or operations that involve I/O or memory allocation.28
Table 2: Spinlock vs. Mutex Selection Guide
Scenario
Lock Type
Reason
Protecting data inside an Interrupt Handler
Spinlock
Interrupt handlers cannot sleep.
Protecting a hardware register (quick access)
Spinlock
Low overhead, fast execution.
Protecting a linked list traversal
Mutex
Traversal might take time; allows other tasks to run.
Calling kmalloc(..., GFP_KERNEL)
Mutex
kmalloc may sleep; spinlocks forbid sleeping.
Copying data to/from user space
Mutex
User memory access may cause page faults (sleep).

7.3 Atomic Operations
For simple integer variables (like reference counters), full locks are too expensive. The kernel provides atomic_t types and functions like atomic_inc, atomic_dec, and atomic_add. These compile to atomic CPU instructions (e.g., LOCK XADD on x86), guaranteeing thread safety without the overhead of locking or sleeping.31
8. Advanced I/O Control: The Evolution of ioctl
While read and write handle data flow, device configuration is handled by ioctl (Input/Output Control). The implementation of this interface has undergone significant modernization to improve kernel scalability.
8.1 The Big Kernel Lock (BKL) Era
In older kernel versions (pre-2.6.36), the ioctl file operation was invoked under the protection of the Big Kernel Lock (BKL). The BKL was a global lock that prevented multiple processes from running kernel code simultaneously in certain paths. While this simplified driver development (no need for internal locking), it devastated performance on multi-core systems because only one ioctl could run at a time across the entire system.32
8.2 The Modern Standard: unlocked_ioctl
To remove the BKL bottleneck, the .ioctl field was removed from file_operations and replaced with .unlocked_ioctl.
Implication: The kernel calls this function without holding any locks.
Responsibility: The driver developer is now fully responsible for implementing their own locking strategies (using mutexes or spinlocks) to protect shared data. This allows fine-grained locking: two users can call ioctl on two different devices simultaneously without blocking each other.33
8.3 Handling 32-bit Compatibility: compat_ioctl
A common challenge in modern 64-bit kernels is supporting 32-bit user-space applications. The memory layout of structures (padding, pointer sizes) differs between 32-bit and 64-bit architectures.
If a driver accepts a C structure via ioctl, a 32-bit app will send a structure different from what the 64-bit driver expects.
Solution: The .compat_ioctl entry point allows the driver to define a specific handler for 32-bit processes. This function typically translates the 32-bit structure into the 64-bit native format before processing.32
9. Managed Device Resources (Devres)
One of the most persistent sources of bugs in Linux drivers is error handling in the initialization (probe) path. If a driver acquires three resources (e.g., memory, IRQ, I/O region) and fails to acquire the fourth, it must release the first three in reverse order. Doing this manually with goto labels is error-prone.
9.1 The devm_ Paradigm
The Devres (Device Resource) framework automates this. It provides "managed" versions of resource allocation functions, prefixed with devm_.
kzalloc() $\rightarrow$ devm_kzalloc()
request_irq() $\rightarrow$ devm_request_irq()
iomap() $\rightarrow$ devm_iomap()
Mechanism:
When a resource is allocated with a devm_ function, the kernel attaches a record of that allocation to the struct device. If the driver's probe function fails or when the device is detached (unplugged), the kernel automatically walks the list of resources and frees them. This eliminates the need for explicit cleanup code in the remove function or error paths, significantly reducing the surface area for memory leaks.35
9.2 The devm_kzalloc Trap with cdev
While Devres is powerful, it contains a subtle trap documented in kernel discussions.37 A common mistake is allocating the struct cdev itself using devm_kzalloc.
The Issue: The lifetime of a character device is not tied strictly to the hardware presence. A user application might keep the file /dev/mydevice open even after the physical device is unplugged.
The Crash: If the driver uses devm_kzalloc for the cdev, the memory will be freed immediately when the device is unplugged (driver detach). However, the cdev is still referenced by the open file descriptor. When the user eventually closes the file, the kernel tries to access the (now freed) cdev, causing a Use-After-Free (UAF) crash.
Best Practice: cdev structures should often be reference-counted separately or embedded in structures whose lifetime is carefully managed to outlive the physical device connection if necessary.37
10. Rust in the Linux Kernel: The New Frontier
In a historic shift, Linux 6.1 introduced initial support for the Rust programming language, making it the second official language of the kernel after C. This integration aims to mitigate the class of memory safety vulnerabilities (buffer overflows, use-after-free) that plague C drivers.
10.1 Safety Guarantees and Architecture
Rust brings compile-time memory safety to the kernel.
Ownership Model: Rust's borrow checker ensures that data races and pointer aliasing bugs are caught at compile time.
Abstractions: The "Rust for Linux" project provides safe wrappers around kernel C APIs. For example, the Mutex type in Rust ensures that the data it protects cannot be accessed unless the lock is held, enforcing correct locking discipline via the type system.7
10.2 Integration Challenges
Developing a Rust driver (as of 2025) requires a hybrid workflow.
FFI and Bindgen: Rust needs to interact with the existing millions of lines of C code. The tool bindgen reads C header files and generates unsafe Rust bindings. Driver authors then wrap these unsafe bindings in safe Rust abstractions.
Build System: The build process involves a build.rs file and strict version requirements for the Rust compiler (rustc) and LLVM, often necessitating newer toolchains than what some enterprise distributions provide by default.39
While still maturing, the presence of Rust signifies a long-term transition toward safer kernel engineering practices.
11. Debugging, Logging, and Style
The constraints of Ring 0 mean standard debuggers (like GDB) are difficult to use. Logging remains the primary debugging mechanism.
11.1 The printk Ring Buffer
The kernel function printk is the analog to printf. It writes to a circular buffer (the ring buffer).
Log Levels: Messages must be tagged with severity levels (e.g., KERN_INFO, KERN_ERR). It is best practice to use the helper macros: pr_info(), pr_err(), pr_warn(), which ensure proper formatting.
Visibility: The dmesg command displays the contents of the ring buffer. The visibility of messages on the system console is controlled by the console_loglevel parameter.41
11.2 Checkpatch and Coding Style
The Linux kernel community enforces a rigid coding style.
checkpatch.pl: This script (found in scripts/checkpatch.pl) analyzes patches and source files for style violations. It checks for indentation (tabs vs. spaces), comment style, and the usage of deprecated APIs.
Compliance: Running checkpatch.pl is mandatory before submitting any code to the mainline kernel. It categorizes issues into errors (must fix), warnings (should fix), and checks (suggestions).43
12. Conclusion
Developing Linux kernel drivers is an exacting engineering discipline that demands a holistic understanding of the operating system's architecture. It requires navigating the constraints of the 3G/1G split, managing memory without the safety net of garbage collection, and orchestrating concurrency across multiple CPUs.
The evolution of the kernel—from the BKL to fine-grained locking, from manual cleanup to Devres, and now from C to Rust—demonstrates a relentless pursuit of performance and reliability. By adhering to the architectural patterns detailed in this report, specifically regarding proper cdev registration, safe user-space memory access, and correct locking hierarchy, developers can contribute robust modules that maintain the stability and integrity of the Linux ecosystem.
Works cited
My First Linux Kernel Module, accessed January 7, 2026, https://medium.com/@ganga.jaiswal/my-first-linux-kernel-module-b0de91a3c492
Can I say copy_to_user()/copy_from_user() is a memcpy with access_ok()? - Stack Overflow, accessed January 7, 2026, https://stackoverflow.com/questions/40415046/can-i-say-copy-to-user-copy-from-user-is-a-memcpy-with-access-ok
User space memory access from the Linux kernel - IBM Developer, accessed January 7, 2026, https://developer.ibm.com/articles/l-kernel-memory-access/
Setting Up Linux Development Environment - zenarmor.com, accessed January 7, 2026, https://www.zenarmor.com/docs/linux-tutorials/setting-up-linux-development-environment
Set up a Development Environment to Build the Kernel - SOF Project documentation, accessed January 7, 2026, https://thesofproject.github.io/latest/getting_started/setup_linux/prepare_build_environment.html
Building a Custom Kernel - Fedora Docs, accessed January 7, 2026, https://docs.fedoraproject.org/en-US/quick-docs/kernel-build-custom/
Quick Start - The Linux Kernel documentation, accessed January 7, 2026, https://docs.kernel.org/rust/quick-start.html
Building External Modules — The Linux Kernel documentation, accessed January 7, 2026, https://docs.kernel.org/kbuild/modules.html
Beginning Linux Kernel Development - My First Linux Kernel Module (LKM) - Hello World, accessed January 7, 2026, https://www.securitynik.com/2021/07/beginning-linux-kernel-development-my.html
Linux Kernel Makefiles, accessed January 7, 2026, https://www.infradead.org/~mchehab/kernel_docs/kbuild/makefiles.html
Anatomy of the Linux loadable kernel module, accessed January 7, 2026, https://terenceli.github.io/%E6%8A%80%E6%9C%AF/2018/06/02/linux-loadable-module
The Anatomy of a Kernel Module: Module Init and Exit Functions - DoHost, accessed January 7, 2026, https://dohost.us/index.php/2025/11/08/the-anatomy-of-a-kernel-module-module-init-and-exit-functions/
Linux Kernel Module Programming — Simplest example | by Sachith Muhandiram - Medium, accessed January 7, 2026, https://sachithmuhandiram.medium.com/linux-kernel-module-programming-simplest-example-c45f2d1b32a7
4. Kernel Modules — Linux Kernel Workbook 1.0 documentation - Read the Docs, accessed January 7, 2026, https://lkw.readthedocs.io/en/latest/doc/03_kernel_modules.html
c - __init and __exit attributes for loadable kernel modules - Stack Overflow, accessed January 7, 2026, https://stackoverflow.com/questions/75782076/init-and-exit-attributes-for-loadable-kernel-modules
Simple Linux character device driver - Oleg Kutkov, accessed January 7, 2026, https://olegkutkov.me/2018/03/14/simple-linux-character-device-driver/
Cdev structure and File Operations – Linux Device Driver Tutorial Part 6 - EmbeTronicX, accessed January 7, 2026, https://embetronicx.com/tutorials/linux/device-drivers/cdev-structure-and-file-operations-of-character-drivers/
Linux cdev vs register_chrdev - driver - Stack Overflow, accessed January 7, 2026, https://stackoverflow.com/questions/27174404/linux-cdev-vs-register-chrdev
Character device drivers — The Linux Kernel documentation, accessed January 7, 2026, https://linux-kernel-labs.github.io/refs/heads/master/labs/device_drivers.html
Character Drivers and How to Register One in the Kernel | by Creata Kulkarni - Medium, accessed January 7, 2026, https://medium.com/@CreataKulkarni/char-drivers-and-how-to-register-one-into-the-kernel-part-1-80c985527fb6
Build Error Related to Calling 'class_create' Function - Meinberg Knowledge Base, accessed January 7, 2026, https://kb.meinbergglobal.com/kb/driver_software/driver_software_for_linux/troubleshooting_build_problems/build_error_related_to_calling_class_create_function
Solved: Re: redhat9 5.14.0 kernal header changes break sepdk build - Intel Community, accessed January 7, 2026, https://community.intel.com/t5/Analyzers/redhat9-5-14-0-kernal-header-changes-break-sepdk-build/m-p/1605880
Linux Device Model — The Linux Kernel documentation, accessed January 7, 2026, https://linux-kernel-labs.github.io/refs/heads/master/labs/device_model.html?highlight=device_create
Learn How to Create Device Files for Character Drivers in Linux - EmbeTronicX, accessed January 7, 2026, https://embetronicx.com/tutorials/linux/device-drivers/device-file-creation-for-character-drivers/
Device drivers infrastructure — The Linux Kernel documentation, accessed January 7, 2026, https://www.kernel.org/doc/html/v4.12/driver-api/infrastructure.html
Character Device Files - Creation & Operations | Introduction, accessed January 7, 2026, https://sysplay.github.io/books/LinuxDrivers/book/Content/Part05.html
Memory Allocation Guide - The Linux Kernel documentation, accessed January 7, 2026, https://docs.kernel.org/core-api/memory-allocation.html
Unreliable Guide To Locking — The Linux Kernel documentation, accessed January 7, 2026, https://www.kernel.org/doc/html/v4.13/kernel-hacking/locking.html
The Complicated History of a Simple Linux Kernel API - grsecurity, accessed January 7, 2026, https://grsecurity.net/complicated_history_simple_linux_kernel_api.php
Kernel Locking Techniques - UT Austin Computer Science, accessed January 7, 2026, https://www.cs.utexas.edu/~pingali/CS395T/2012sp/papers/LinuxLocks.htm
The Linux Kernel Locking API and Shared Objects | by Packt | Geek Culture - Medium, accessed January 7, 2026, https://medium.com/geekculture/the-linux-kernel-locking-api-and-shared-objects-1169c2ae88ff
What is the difference between ioctl(), unlocked_ioctl() and compat_ioctl()?, accessed January 7, 2026, https://unix.stackexchange.com/questions/4711/what-is-the-difference-between-ioctl-unlocked-ioctl-and-compat-ioctl
2416 Development Record 4: Difference between ioctl and unlocked_ioctl - EEWorld, accessed January 7, 2026, https://en.eeworld.com.cn/news/mcu/eic447166.html
[JANITOR PROPOSAL] Switch ioctl functions to ->unlocked_ioctl, accessed January 7, 2026, https://linux.kernel.narkive.com/of82FCBC/janitor-proposal-switch-ioctl-functions-to-unlocked-ioctl
Devres - Managed Device Resource - The Linux Kernel documentation, accessed January 7, 2026, https://docs.kernel.org/driver-api/driver-model/devres.html
The Right Way: Managed Resource Allocation in Linux Device Drivers - Haifux, accessed January 7, 2026, http://www.haifux.org/lectures/323/haifux-devres.pdf
Why is devm_kzalloc() harmful and what can we do about it - Linux Plumbers Conference, accessed January 7, 2026, https://lpc.events/event/16/contributions/1227/
Kernel Recipes 2025 - So you want to write a driver in Rust? - YouTube, accessed January 7, 2026, https://www.youtube.com/watch?v=sGbNzu_5FUI
Writing a Simple Driver in Rust - Pavel Yosifovich, accessed January 7, 2026, https://scorpiosoftware.net/2025/02/08/writing-a-simple-driver-in-rust/
Linux Driver Development with Rust: Benefits, Challenges, and a Practical Example, accessed January 7, 2026, https://www.apriorit.com/dev-blog/rust-for-linux-driver
Let's code a Linux Driver: 6- printk log levels - YouTube, accessed January 7, 2026, https://www.youtube.com/watch?v=Wr6BoRsSvws
Chapter 11. Getting started with kernel logging | Managing, monitoring, and updating the kernel | Red Hat Enterprise Linux | 9, accessed January 7, 2026, https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/managing_monitoring_and_updating_the_kernel/getting-started-with-kernel-logging_managing-monitoring-and-updating-the-kernel
Checkpatch - The Linux Kernel documentation, accessed January 7, 2026, https://docs.kernel.org/dev-tools/checkpatch.html
How to make a GNU/Linux kernel patch? - Reliable Embedded Systems, accessed January 7, 2026, https://www.reliableembeddedsystems.com/pdfs/WE-2.1_Berger-paper.pdf
