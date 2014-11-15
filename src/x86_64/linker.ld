OUTPUT_FORMAT("elf64-x86-64", "elf64-x86-64",
        "elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

SECTIONS
{

  /* somehow needed for grub to recognize the multiboot header */
  . = 2M + SIZEOF_HEADERS;
  kernel_start_symbol_table_entry = .;

  .boot : 
  {
    *(.multiboot_header)
    *(.boot)
    *(.isr)
  }

  .text :
  {
    *(.text)
    *(.text.*)
  }

  .rodata :
  {
    *(.rodata)
    *(.rodata.*)
  }

  .data :
  {
    *(.data)
  }

  .bss :
  {
   *(.bss)
   *(COMMON)
  }

  .got.plt : 
  {
    *(.got.plt)
  }

  /DISCARD/ : 
  {
    *(.comment)
    *(.debug_*)
    *(.eh_frame)
  }

  kernel_end_symbol_table_entry = .;

}


