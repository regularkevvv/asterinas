/* SPDX-License-Identifier: MPL-2.0 */

.text

# Sets `size` bytes of memory at `dst` to the byte value given by `value`.
# This function works with exception handling and can recover from a page fault.
#
# Returns number of bytes that failed to set.
.global __memset_fallible
.type __memset_fallible, @function
__memset_fallible: # (dst: *mut u8, value: u8, size: usize) -> usize
    mrs     x9, id_aa64mmfr1_el1
    ubfx    x9, x9, #20, #4
    cbz     x9, 1f
    # `msr pan, #0`, encoded directly so generic ARMv8.0 builds accept it.
    .inst   0xd500409f
1:
memset_loop:
    cbz     x2, memset_exit
memset_store:
    strb    w1, [x0], #1
    add     x2, x2, #-1
    b       memset_loop
memset_exit:
    mov     x0, x2
    cbz     x9, 2f
    # `msr pan, #1`.
    .inst   0xd500419f
2:
    ret
.size __memset_fallible, .-__memset_fallible

.pushsection .ex_table, "a", @progbits
    .balign 8
    .quad memset_store
    .quad memset_exit
.popsection
