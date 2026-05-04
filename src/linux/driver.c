#include <linux/kernel.h>
#include <linux/module.h>

MODULE_LICENSE("Daniel's magic license");
MODULE_DESCRIPTION("This is the most elaborate kernel driver that has ever been devised. Pure art.");

int init_module()
{
    printk(KERN_INFO "Hello Daniel\n");

    return 0;
}

void cleanup_module()
{
    printk(KERN_INFO "Bye bye, love you!\n");
}
