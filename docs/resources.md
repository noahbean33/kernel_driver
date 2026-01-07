# Resources

## Kernel Source Browser

- **Bootlin**: <https://bootlin.com/>
- **Elixir Cross Referencer**: <https://elixir.bootlin.com/linux/v6.18.3/source>

## System Information

Get your kernel version:

```bash
uname -r
```

Current development kernel: `6.8.0-90-generic`

## Key Kernel Functions

### copy_from_user

Safely copies data from userspace to kernel space.

**Definition locations:**

- `include/linux/uaccess.h`, line 205 (as a function)
- `tools/virtio/linux/uaccess.h`, line 32 (as a function)

### file_operations

Structure defining file operation callbacks for character devices.

**Definition locations:**

- `include/linux/fs.h`, line 2268 (as a struct)
- `tools/testing/vma/vma_internal.h`, line 303 (as a struct)

## Additional Resources

- [Linux Kernel Documentation](https://www.kernel.org/doc/html/latest/)
- [Linux Device Drivers, 3rd Edition](https://lwn.net/Kernel/LDD3/)
- [Bootlin Training Materials](https://bootlin.com/docs/)
