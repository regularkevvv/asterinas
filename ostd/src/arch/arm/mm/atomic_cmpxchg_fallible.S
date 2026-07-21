/* SPDX-License-Identifier: MPL-2.0 */

.text

# Atomically compares and exchanges a 32-bit integer value. This function works
# with exception handling and can recover from a page fault.
#
# Returns the previous value or `!0u64` if failed to update.
.global __atomic_cmpxchg_fallible
.type __atomic_cmpxchg_fallible, @function
__atomic_cmpxchg_fallible: # (ptr: *mut u32, old_val: u32, new_val: u32) -> u64
    mrs     x9, id_aa64mmfr1_el1
    ubfx    x9, x9, #20, #4
    cbz     x9, 1f
    # `msr pan, #0`, encoded directly so generic ARMv8.0 builds accept it.
    .inst   0xd500409f
1:
cmpxchg_load:
    ldxr    w4, [x0]
    cmp     w1, w4
    b.ne    cmpxchg_done
cmpxchg_store:
    stxr    w5, w2, [x0]
    cbnz    w5, cmpxchg_load
cmpxchg_done:
    mov     w0, w4
    cbz     x9, 2f
    # `msr pan, #1`.
    .inst   0xd500419f
2:
    ret
cmpxchg_fault:
    mov     x0, #-1
    cbz     x9, 3f
    # `msr pan, #1`.
    .inst   0xd500419f
3:
    ret
.size __atomic_cmpxchg_fallible, .-__atomic_cmpxchg_fallible

.pushsection .ex_table, "a", @progbits
    .balign 8
    .quad cmpxchg_load
    .quad cmpxchg_fault
    .quad cmpxchg_store
    .quad cmpxchg_fault
.popsection
