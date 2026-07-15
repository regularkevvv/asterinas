/* SPDX-License-Identifier: MPL-2.0 */

.text

# Copies `size` bytes from `src` to `dst`. This function works with exception
# handling and can recover from a page fault. The source range must not overlap
# with the destination range (In virtual address level. Their corresponding
# physical addresses can be overlapped).
#
# Returns number of bytes that failed to copy.
.global __memcpy_fallible
.type __memcpy_fallible, @function
__memcpy_fallible: # (dst: *mut u8, src: *const u8, size: usize) -> usize
    mrs     x9, id_aa64mmfr1_el1
    ubfx    x9, x9, #20, #4
    cbz     x9, 1f
    # `msr pan, #0`, encoded directly so generic ARMv8.0 builds accept it.
    .inst   0xd500409f
1:
memcpy_loop:
    cbz     x2, memcpy_exit
memcpy_load:
    ldrb    w3, [x1], #1
memcpy_store:
    strb    w3, [x0], #1
    add     x2, x2, #-1
    b       memcpy_loop
memcpy_exit:
    mov     x0, x2
    cbz     x9, 2f
    # `msr pan, #1`.
    .inst   0xd500419f
2:
    ret
.size __memcpy_fallible, .-__memcpy_fallible

.pushsection .ex_table, "a", @progbits
    .balign 8
    .quad memcpy_load
    .quad memcpy_exit
    .quad memcpy_store
    .quad memcpy_exit
.popsection
