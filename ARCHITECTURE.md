# Architecture

## Code Map

### `apps`

Applications.

### `boot`

Here you will find files related to the boot loader, such as the IPL and loader.

- `/boot/pc-bios/fdboot.asm`
  - IPL for floppy
- `/boot/pc-bios/loader.asm`
  - Standard boot loader

### `ext`

Files imported from outside this project.

### `lib`

Common library that spans multiple programs.

- `/lib/megstd/`
  - MEG-OS Standard Library
- `/lib/toeboot/`
  - boot protocol

### `misc`

Miscellaneous.

- `/misc/initrd/`
  - Source of initrd

### `sys`

The kernel and system sources.

- `/sys/kernel/src/arch/`
  - Architecture-specific code

### `tools`

Small tools used for building.

- `/tools/elf2ceef/`
  - Convert format of the kernel
- `/tools/mkfdfs/`
  - Make a floppy disk image
- `/tools/mkinitrd/`
  - Make an initrd image

## Boot Sequence (PC-BIOS)

``` plantuml
@startuml
title BIOS to Kernel
start

partition BIOS {
    :Some initialization processes;
}

partition IPL {
    :load KERNEL.SYS from BootDisk;
    :invoke Loader at the top of the KERNEL.SYS;
}

partition Loader_(Real_Mode) {
    :relocate Loader to ORG_BASE (0000:0800);
    if (check CPU, Memory size and Video mode) then (OK)
    else (NG)
        :error;
        end
    endif
    :set Video mode to SVGA;
    :clear A20 mask;
    :enter to Protected Mode;
}
partition Loader_(Protected_Mode) {
    :Move the INITRD behind KERNEL.SYS to the end of memory;
    :relocate the KERNEL from the INITRD;
    :invoke KERNEL;
}
:Kernel entry point;
stop

@enduml
```

``` plantuml
@startuml
title Kernel Initialization
start
:entry point;
:System::init();
partition System::init() {
    :make main_screen;
    :MemoryManager::init_first();
    :Arch::init();
    :Scheduler::start();
}
split 
    :idle;
    detach
split again
    :System::late_init();
    partition System::late_init() {
        :MemoryManager::late_init();
        :FileManager::init();
        :FontManager::init();
        :WindowManager::init();
        :HidManager::init();
        :Arch::late_init();
    }
    :Shell::main();
    stop
end split

@enduml
```

## Kernel

- TBD

## Memory Manager

- MEG-OS allocates large memory blocks in pages. Smaller memory blocks are allocated with a slab allocator.
- MEG-OS Lite does not support any features of the MMU to protect the system.

## Scheduler

MEG-OS supports five priority-based preemptive multi-threaded schedulers.

- Priority **Real-time** is scheduled with the highest priority and is never preempted.
- The **high**, **normal**, and **low** priorities are each scheduled in a round-robin fashion and will be preempted when the allocated Quantum is consumed.
- Priority **idle** makes the processor idle when all other threads are waiting. It is never scheduled.

## Window System

- TBD

## Hid Manager

- HidManager relays between human interface devices and the window event subsystem
- Keyboard scancodes will be converted to the Usage specified by the USB-HID specification on all platforms

## FileSystem

- TBD

## User Land (Personality)

- TBD

## FAQ

### How does MEG-OS identify the platform at runtime?

The common IPL for floppies identifies the PC98, IBM PC, and FM TOWNS by the value of the code segment when called by the BIOS.
At that time, the code segment value of PC98 can be identified by 1FXX, IBM PC by a smaller value (07C0 or 0000), and FM TOWNS by a larger value.

Note that the identifier "IPL4" is required in the IPL OEM name in order to run on FM TOWNS.
