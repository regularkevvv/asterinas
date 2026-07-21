/* SPDX-License-Identifier: MPL-2.0 */

.text

# Atomically loads a 32-bit integer value. This function works with exception
# handling and can recover from a page fault.
#
# Returns the loaded value or `!0u64` if failed to load.
.global __atomic_load_fallible
.type __atomic_load_fallible, @function
__atomic_load_fallible: # (ptr: *const u32) -> u64;
    mrs     x9, id_aa64mmfr1_el1
    ubfx    x9, x9, #20, #4
    cbz     x9, 1f
    # `msr pan, #0`, encoded directly so generic ARMv8.0 builds accept it.
    .inst   0xd500409f
1:
load:
    ldr     w0, [x0]
    cbz     x9, 2f
    # `msr pan, #1`.
    .inst   0xd500419f
2:
    ret
load_fault:
    mov     x0, #-1
    cbz     x9, 3f
    # `msr pan, #1`.
    .inst   0xd500419f
3:
    ret
.size __atomic_load_fallible, .-__atomic_load_fallible

.pushsection .ex_table, "a", @progbits
    .balign 8
    .quad load
    .quad load_fault
.popsection
