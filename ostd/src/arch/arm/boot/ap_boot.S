/* SPDX-License-Identifier: MPL-2.0 */

// The boot routine executed by application processors (APs) on AArch64.

// Translation Control Register (TCR), matching bsp_boot.S.
TCR_TG_4K              = (2 << 30) | (0 << 14)
TCR_SH_INNER           = (3 << 28) | (3 << 12)
TCR_ORGN_WBWA          = (1 << 26) | (1 << 10)
TCR_IRGN_WBWA          = (1 << 24) | (1 << 8)
TCR_TSZ_48             = (16 << 16) | (16 << 0)
TCR_IPS_48             = 5 << 32

// Memory Attribute Indirection Register (MAIR), matching bsp_boot.S.
MAIR_0WBWA             = 0xff << 0
MAIR_1DEVICE           = 0x00 << 8

// System Control Register (SCTLR).
SCTLR_I                = 1 << 12
SCTLR_C                = 1 << 2
SCTLR_M                = 1 << 0

.section ".ap_boot", "awx", @progbits

.balign 4096
.global ap_boot_start
ap_boot_start:
    // PSCI supplies the logical CPU ID as the context ID in x0.
    mov     x19, x0

    // Asterinas expects the firmware to enter the kernel at EL1.
    mrs     x1, currentel
    cmp     x1, #(0b01 << 2)
1:
    b.ne    1b

    ldr     x1, =(TCR_TG_4K | TCR_SH_INNER | TCR_ORGN_WBWA | TCR_IRGN_WBWA | TCR_TSZ_48 | TCR_IPS_48)
    msr     tcr_el1, x1
    mov     x1, #(MAIR_0WBWA | MAIR_1DEVICE)
    msr     mair_el1, x1

    // The pointer slot is in this identity-mapped section, so it is reachable before the MMU is
    // enabled without relying on a high virtual address.
    adrp    x1, __ap_boot_page_table_pointer
    add     x1, x1, :lo12:__ap_boot_page_table_pointer
    ldr     x2, [x1]
    msr     ttbr0_el1, x2
    msr     ttbr1_el1, x2

    isb
    tlbi    vmalle1
    dsb     sy
    isb

    mrs     x1, sctlr_el1
    mov     x2, #(SCTLR_I | SCTLR_C | SCTLR_M)
    orr     x1, x1, x2
    msr     sctlr_el1, x1
    isb

    // Load this AP's high-virtual-address stack and CPU-local storage pointers.
    adrp    x1, __ap_boot_info_array_pointer
    add     x1, x1, :lo12:__ap_boot_info_array_pointer
    ldr     x1, [x1]
    sub     x2, x19, #1
    lsl     x2, x2, #4
    add     x1, x1, x2
    ldr     x2, [x1]
    mov     sp, x2
    ldr     x2, [x1, #8]
    msr     tpidr_el1, x2

    mov     x29, xzr
    mov     w0, w19
    ldr     x1, =ap_early_entry
    br      x1

2:
    b       2b

.balign 8
.global __ap_boot_page_table_pointer
__ap_boot_page_table_pointer:
    .quad 0
.global __ap_boot_info_array_pointer
__ap_boot_info_array_pointer:
    .quad 0
