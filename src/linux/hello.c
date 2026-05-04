/**
 * Simple Character Device Driver
 * 
 * This module demonstrates basic kernel driver development by creating
 * a character device that allows userspace programs to read and write
 * to a kernel buffer.
 * 
 * Usage:
 *   Load module:   sudo insmod ./hello.ko
 *   Create device: sudo mknod /dev/mydev c 90 0
 *   Write data:    echo "hello" > /dev/mydev
 *   Read data:     cat /dev/mydev
 *   Unload module: sudo rmmod hello
 */

#include <linux/init.h>     // Module initialization macros
#include <linux/module.h>   // Core module support
#include <linux/kernel.h>   // Kernel logging (printk)
#include <linux/fs.h>       // File operations structure
#include <linux/string.h>   // String manipulation
#include <linux/uaccess.h>  // User space memory access (copy_to_user, copy_from_user)

/* Module metadata - shows up in modinfo */
MODULE_LICENSE("GPL");
MODULE_AUTHOR("NOAH");
MODULE_DESCRIPTION("HELLO WORLD");
MODULE_VERSION("1.0");

/* Kernel buffer to store data exchanged with userspace (100 bytes) */
static char kern_buf[100];

/* Forward declarations of file operation functions */
static int dev_open(struct inode *inode, struct file *file);
static int dev_release(struct inode *inode, struct file *file);
static ssize_t dev_read(struct file *file, char __user *buf, size_t len, loff_t *off);
static ssize_t dev_write(struct file *file, const char __user *buf, size_t len, loff_t *off);

/**
 * File operations structure
 * Defines the callbacks for file operations on our device
 */
static struct file_operations fops = {
    .read = dev_read,      // Called when userspace reads from the device
    .write = dev_write,    // Called when userspace writes to the device
    .open = dev_open,      // Called when the device file is opened
    .release = dev_release // Called when the device file is closed
};

/**
 * Module initialization function
 * Called when the module is loaded into the kernel (insmod)
 * 
 * Registers a character device with major number 90
 * Returns: 0 on success, negative error code on failure
 */
static int __init helloworld_init(void) 
{
    int t = register_chrdev(90, "mydev", &fops);
    if (t < 0) {
        printk(KERN_ERR "Problem registering character device\n");
        return -EIO;
    }

    printk(KERN_INFO "Hello World!\n");
    return 0;
}

/**
 * Device open function
 * Called when userspace opens the device file (e.g., cat /dev/mydev)
 * 
 * Returns: 0 on success
 */
static int dev_open(struct inode *inode, struct file *file)
{
    printk(KERN_INFO "Device Opened\n");
    return 0;
}

/**
 * Device read function
 * Called when userspace reads from the device (e.g., cat /dev/mydev)
 * Copies data from kernel buffer to userspace buffer
 * 
 * @buf: Userspace buffer to copy data into
 * @len: Number of bytes requested by userspace
 * @off: Current file position offset
 * 
 * Returns: Number of bytes read, 0 on EOF, negative error code on failure
 */
static ssize_t dev_read(struct file *file, char __user *buf, size_t len, loff_t *off)
{
    size_t to_read;
    unsigned long not_copied;

    /* EOF check - if we've read past the buffer, return 0 */
    if (*off >= sizeof(kern_buf))
    {
        return 0;
    }

    /* Calculate how many bytes we can read from current offset */
    to_read = sizeof(kern_buf) - *off;
    if (len > to_read)
    {
        len = to_read;
    }

    /* Copy data from kernel space to user space */
    not_copied = copy_to_user(buf, kern_buf + *off, len);
    if (not_copied)
    {
        return -EFAULT; // Bad address
    }

    /* Update file position and return bytes read */
    *off += len;
    return len;
}

/**
 * Device write function
 * Called when userspace writes to the device (e.g., echo "data" > /dev/mydev)
 * Copies data from userspace buffer to kernel buffer
 * 
 * @buf: Userspace buffer containing data to write
 * @len: Number of bytes to write
 * @off: Current file position offset
 * 
 * Returns: Number of bytes written, negative error code on failure
 */
static ssize_t dev_write(struct file *file, const char __user *buf, size_t len, loff_t *off)
{
    unsigned long not_copied;

    /* Reject writes that are too large (need space for null terminator) */
    if (len >= sizeof(kern_buf))
    {
        return -EINVAL; // Invalid argument
    }

    /* Clear the buffer before writing new data */
    memset(kern_buf, 0, sizeof(kern_buf));
    
    /* Copy data from user space to kernel space */
    not_copied = copy_from_user(kern_buf, buf, len);
    if (not_copied)
    {
        return -EFAULT; // Bad address
    }

    /* Null-terminate the string */
    kern_buf[len] = '\0';
    *off = len;
    return len;
}

/**
 * Device release function
 * Called when userspace closes the device file
 * 
 * Returns: 0 on success
 */
static int dev_release(struct inode *inode, struct file *file)
{
    printk(KERN_INFO "Device Closed\n");
    return 0;
}

/**
 * Module cleanup function
 * Called when the module is unloaded from the kernel (rmmod)
 * Unregisters the character device
 */
static void __exit helloworld_exit(void)
{
    unregister_chrdev(90, "mydev");
    printk(KERN_INFO "Goodbye World!\n");
}

/* Register init and exit functions */
module_init(helloworld_init);
module_exit(helloworld_exit);