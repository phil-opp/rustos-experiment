[BITS 64]
section .isr

%macro push_xmm 1
    sub rsp, 16
    movdqu [rsp], xmm%1
%endmacro
%macro pop_xmm 1
    movdqu  xmm%1, [rsp]
    add     rsp, 16
%endmacro

push_registers_and_call_handler:
    push rbx
    push rcx
    push rdx
    push rbp
    push rsi
    push rdi

    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    ;push xmm registers
    %assign i 0
    %rep 16
        push_xmm i
    %assign i i+1
    %endrep

    push qword [fs:0x70]

    mov rdi, [rsp + 384]   ;interrupt number
    mov rsi, [rsp + 392]   ;error code
    mov rdx, rsp        ;stack pointer


    call rax

pop_registers_and_iretq:
    pop qword [fs:0x70] ;restore stack limit

    ;pop xmm registers
    %assign i 15
    %rep 16
        pop_xmm i
    %assign i i-1
    %endrep

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8

    pop rdi
    pop rsi
    pop rbp
    pop rdx
    pop rcx
    pop rbx
    pop rax
    add rsp, 16 ;remove interrupt number and error code

    iretq

.hang:
    cli
    hlt
    jmp .hang



; special handlers

%macro HANDLER_WITH_ERRCODE 2
    _handler_%1:
        push qword %1
        push rax
        mov rax, %2
        jmp push_registers_and_call_handler
%endmacro

extern interrupt_handler
extern pagefault_handler

%define H8
HANDLER_WITH_ERRCODE 8, interrupt_handler
%define H10
HANDLER_WITH_ERRCODE 10, interrupt_handler
%define H11
HANDLER_WITH_ERRCODE 11, interrupt_handler
%define H12
HANDLER_WITH_ERRCODE 12, interrupt_handler
%define H13
HANDLER_WITH_ERRCODE 13, interrupt_handler
%define H14
_handler_14: ;pagefault
    sub rsp, 8  ;make room for cr2 (replaces interrupt number)
    push rax

    add rsp, 16     ;write cr2 on stack before rax
    mov rax, cr2
    push rax    
    sub rsp, 8      ;move rsp to tos again

    mov rax, pagefault_handler
    jmp push_registers_and_call_handler

%define H33
_handler_33: ;keyboard
    sub rsp, 8 ;room for keyboard code
    push qword 33 ;interrupt number
    push rax

    mov rax, 0
    in al, 0x60
    mov [rsp + 16], rax

    mov rax, interrupt_handler
    jmp push_registers_and_call_handler


;other handlers (standard)

%macro HANDLER 1
    %ifndef H%1
    _handler_%1:
        push qword 0 ;dummy error code
        push qword %1
        push rax
        mov rax, interrupt_handler
        jmp push_registers_and_call_handler
    %endif
%endmacro

%assign i 0
%rep 256
    HANDLER i
%assign i i+1
%endrep

;IDT

%macro IDT_ENTRY 1
    ;TODO: remove the word data exceeds bounds warning
    DW (_handler_%1-0x200000)  ;offset_low
    DW 0x8              ;text segment
    DB 0                ;zero1
    DB 0x8e             ;type_addr: present+interrupt_gate
    DW 0x20             ;offset_middle
    DQ 0                ;offset_high and zero2
%endmacro

%macro not_present_IDT_ENTRY 1
    ;TODO: remove the word data exceeds bounds warning
    DW (_handler_%1-0x200000)  ;offset_low
    DW 0x8              ;text segment
    DB 0                ;zero1
    DB 0x0e             ;type_addr: present+interrupt_gate
    DW 0x20             ;offset_middle
    DQ 0                ;offset_high and zero2
%endmacro

IDT:
    %assign i 0
    %rep 256
        IDT_ENTRY i
    %assign i i+1
    %endrep

global idt_pointer
idt_pointer:
    DW 4095 ;limit
    DQ IDT

InterruptStackBottom:
times 0x8000 db 0
InterruptStack: