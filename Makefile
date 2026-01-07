obj-m += hello.o

# Specify source directory
hello-objs := src/hello.o

KDIR := /lib/modules/$(shell uname -r)/build
PWD := $(shell pwd)

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	rm -f src/*.o src/*.mod src/*.mod.c src/.*.cmd src/.*.o.d