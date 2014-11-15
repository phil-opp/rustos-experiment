BITS 32

global _start

[section .multiboot_header]
align 8
MBHdr:
    dd 0xe85250d6           ;magic number (multiboot 2)
    dd 0                    ;architecture 0 (protected mode i386)
    dd MBHdrEnd - MBHdr     ;header length
    dd 0xffffffff - (0xe85250d6 + 0 + (MBHdrEnd - MBHdr)) + 1 ;checksum

    ;…tags…

    ;end of tags
    dw 0,0
    dd 8
MBHdrEnd:

[section .boot]
_start:
    ;load new gdt (we can't trust the rust gdt)
    mov eax, Gdt32Pointer
    lgdt [eax]

    mov esp, Stack  ;load stack
    push 0x08       ;code segment
    push .Gdt32PointerReady
    retf

.Gdt32PointerReady:
    ;reload segment registers
    mov eax, 0x10   ;data gdt segment
    mov ds, ax
    mov ss, ax

.SetupSSE:
    ;now enable SSE and the like
    mov eax, cr0
    and ax, 0xFFFB      ;clear coprocessor emulation CR0.EM
    or ax, 0x2          ;set coprocessor monitoring  CR0.MP
    mov cr0, eax
    mov eax, cr4
    or ax, 3 << 9       ;set CR4.OSFXSR and CR4.OSXMMEXCPT at the same time
    mov cr4, eax

.SetupPagingAndLongMode:
    mov eax, P3
    or eax, 1
    mov [P4], eax
 
    mov eax, P2
    or eax, 1
    mov [P3], eax

    ;identity map first 8MB (rw)

    mov eax, P1_0
    or eax, 1
    mov [P2], eax

    mov eax, P1_1
    or eax, 1
    mov [P2 + 8], eax

    mov eax, P1_2
    or eax, 1
    mov [P2 + 16], eax

    mov eax, P1_3
    or eax, 1
    mov [P2 + 24], eax

    mov edi, P1_0
    mov eax, 0x000003
    mov ecx, 512*4  ;4 p1 tables
.fill_p1:
    mov dword [edi], eax
    mov dword [edi + 4], 0
    add edi, 8
    add eax, 0x1000
    sub ecx, 1
    jnz .fill_p1

    mov dword [P1_0], 0
    mov dword [P1_0 + 4], 0

    ;recursive map p4
    mov eax, P4
    or eax, 1
    mov [P4 + 511*8], eax
 
    ; Load CR3 with P4
    mov eax, P4
    mov cr3, eax
 
    ; Enable PAE
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
 
    ; Enable Long Mode in the MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr
 
    ; Enable Paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ; load 64bit GDT
    mov eax, Gdt64Pointer
    lgdt [eax]

    push 0x08
    push .Gdt64PointerReady
    retf

[BITS 64]
.Gdt64PointerReady:
    ;reload segment registers
    mov eax, 0x10   ;ring 0 data gdt segment
    mov ss, ax
    mov eax, 0x20   ;ring 3 data gdt segment
    mov ds, ax
    mov es, ax

    ;set fs
    mov edx, 0
    mov eax, fsStruct
    mov ecx, 0xC0000100
    wrmsr
    ;set gs
    mov edx, 0
    mov eax, gsStruct
    mov ecx, 0xC0000101
    wrmsr

.remap_PIC:
    in al, 0x21                   ; save pic1 mask
    mov cl, al    
    in al, 0xA1                   ; save pic2 mask
    mov ch, al

    mov al, 0x11
    out 0x20, al                ; send initialize command to pic1
    out 0xA0, al                ; send initialize command to pic2

    mov al, 0x20
    out 0x21, al                ; set vector offset of pic1 to 0x20
    mov al, 0x28           
    out 0xA1, al                ; set vector offset of pic2 to 0x28           

    mov al, 4
    out 0x21, al                   ; tell pic1 that there is a slave PIC at IRQ2 (0000 0100)
    mov al, 2
    out 0xA1, al                   ; tell pic2 its cascade identity (0000 0010)

    mov al, 0x1
    out 0x21, al                 ; 8086 mode for pic1
    out 0xA1, al                 ; 8086 mode for pic2

    mov al, cl
    out 0x21, al                  ; restore pic1 mask
    mov al, ch
    out 0xA1, al                  ; restore pic2 mask

.reprogram_timer:
    mov rcx, 1193180/100        ;divisor (100 Hz)
    
    mov al, 0x36
    out 0x43, al                ;set channel 0 data register + mode bits

    mov al, cl 
    out 0x40, al                ;set low divisor byte

    mov al, ch
    out 0x40, al                ;set high divisor byte

.64Bit:
    extern idt_pointer
    lidt [idt_pointer]

    ;set stack limit (20 * 1024 red zone + 1024 just to be safe :))
    ;see http://doc.rust-lang.org/rustrt/stack/ + src
    mov qword [fs:0x70], StackBottom + 0x6000

    mov rdi, rbx ;multiboot structure

    extern main 
    call main

    call clear_screen_green

.hang:
    cli
    hlt
    jmp .hang

clear_screen_green:
    mov edi, 0xB8000              ; Set the destination index to 0xB8000.
    mov rax, 0x2F202F202F202F20   ; Set the A-register to 0x1F201F201F201F20.
    mov ecx, 500                  ; Set the C-register to 500.
    rep stosq                     ; Clear the screen.
    ret

[BITS 32]

Gdt32:
    DQ  0x0000000000000000
    DQ  0x00CF9A000000FFFF
    DQ  0x00CF92000000FFFF
Gdt64:
    DQ  0x0000000000000000
    DQ  0x00A09A0000000000  ;ring 0 code
    DQ  0x00A0920000000000  ;ring 0 data
    DQ  0x00A0FA0000000000  ;ring 3 code
    DQ  0x00A0F20000000000  ;ring 3 data
 
Gdt32Pointer:
    DW  23
    DD  Gdt32
 
Gdt64Pointer:
    DW  39
    DD  Gdt64
    DD  0

[section .data]

fsStruct:
times 0x100 db 0
gsStruct:
times 0x100 db 0

;stack
StackBottom:
times 0x8000 db 0
align 0x1000        ;use align bytes as stack
Stack:

P4:
times 0x1000 db 0
P3:
times 0x1000 db 0
P2:
times 0x1000 db 0
P1_0:
times 0x1000 db 0
P1_1:
times 0x1000 db 0
P1_2:
times 0x1000 db 0
P1_3:
times 0x1000 db 0

; - - - - - - - - - - - - - - - - - - - -
; needed by linker for division (why??)
; - - - - - - - - - - - - - - - - - - - -

[section .text]

[BITS 64]

global fmod
fmod: jmp $

global fmodf
fmodf: jmp $

global floorf
floorf: jmp $

global ceilf
ceilf: jmp $

global roundf
roundf: jmp $

global truncf
truncf: jmp $

global fmaf
fmaf: jmp $

global __powisf2
__powisf2: jmp $

global powf
powf: jmp $

global expf
expf: jmp $

global exp2f
exp2f: jmp $

global logf
logf: jmp $

global log2f
log2f: jmp $

global log10f
log10f: jmp $

global floor
floor: jmp $

global ceil
ceil: jmp $

global round
round: jmp $

global trunc
trunc: jmp $

global fma
fma: jmp $

global pow
pow: jmp $

global __powidf2
__powidf2: jmp $

global exp
exp: jmp $

global exp2
exp2: jmp $

global log
log: jmp $

global log2
log2: jmp $

global log10
log10: jmp $

global fdimf
fdimf: jmp $

global fdim
fdim: jmp $