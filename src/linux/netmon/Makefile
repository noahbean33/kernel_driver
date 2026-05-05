KDIR ?= /lib/modules/$(shell uname -r)/build
MODULE_NAME := netmon
obj-m := $(MODULE_NAME).o
CC := clang

all:
	make -C $(KDIR) M=$(PWD) modules CC=$(CC)

clean:
	make -C $(KDIR) M=$(PWD) clean
