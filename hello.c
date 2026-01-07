// move this into the kernel with: sudo insmod ./hello.ko

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("NOAH");
MODULE_DESCRIPTION("HELLO WORLD");
MODULE_VERSION("1.0");

static int __init helloworld_init(void) 
{
    printk(KERN_INFO "Hello World!\n");
    return 0;
}


static void __exit helloworld_exit(void)
{
    printk(KERN_INFO "Goodbye World!\n");
}

module_init(helloworld_init);
module_exit(helloworld_exit);