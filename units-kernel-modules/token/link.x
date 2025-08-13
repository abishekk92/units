/* Linker script for UNITS kernel modules */

ENTRY(_start)

MEMORY
{
    /* Kernel modules are loaded at a fixed address in VM memory */
    RAM : ORIGIN = 0x80000000, LENGTH = 64M
}

SECTIONS
{
    . = ORIGIN(RAM);
    
    /* Code section */
    .text : {
        *(.text._start)
        *(.text .text.*)
    } > RAM
    
    /* Read-only data */
    .rodata : ALIGN(4) {
        *(.rodata .rodata.*)
    } > RAM
    
    /* Data section */
    .data : ALIGN(4) {
        *(.data .data.*)
    } > RAM
    
    /* BSS section */
    .bss : ALIGN(4) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > RAM
    
    /* End of used memory */
    . = ALIGN(4);
    __heap_start = .;
    
    /* Discard debug sections */
    /DISCARD/ : {
        *(.comment)
        *(.debug*)
        *(.eh_frame)
    }
}