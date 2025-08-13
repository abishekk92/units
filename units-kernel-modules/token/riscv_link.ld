/* RISC-V linker script for kernel modules */

OUTPUT_FORMAT("elf64-littleriscv", "elf64-littleriscv", "elf64-littleriscv")
OUTPUT_ARCH(riscv)
ENTRY(main)

MEMORY
{
    /* Kernel modules are loaded at a fixed address in VM memory */
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 64M
}

SECTIONS
{
    . = ORIGIN(RAM);
    
    .text : {
        *(.text.startup)
        *(.text)
        *(.text.*)
    } > RAM
    
    .rodata : {
        *(.rodata)
        *(.rodata.*)
    } > RAM
    
    .data : {
        *(.data)
        *(.data.*)
    } > RAM
    
    .bss : {
        __bss_start = .;
        *(.bss)
        *(.bss.*)
        *(COMMON)
        __bss_end = .;
    } > RAM
    
    /DISCARD/ : {
        *(.comment)
        *(.debug*)
        *(.eh_frame)
    }
}