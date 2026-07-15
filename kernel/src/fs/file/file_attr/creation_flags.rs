// SPDX-License-Identifier: MPL-2.0

use bitflags::bitflags;

// Linux AArch64 keeps the AArch32-compatible values for these flags. Other
// supported 64-bit architectures use the asm-generic/x86_64 layout.
#[cfg(target_arch = "aarch64")]
const O_DIRECTORY_BITS: u32 = 1 << 14;
#[cfg(not(target_arch = "aarch64"))]
const O_DIRECTORY_BITS: u32 = 1 << 16;
#[cfg(target_arch = "aarch64")]
const O_NOFOLLOW_BITS: u32 = 1 << 15;
#[cfg(not(target_arch = "aarch64"))]
const O_NOFOLLOW_BITS: u32 = 1 << 17;

bitflags! {
    pub struct CreationFlags: u32 {
        /// create file if it does not exist
        const O_CREAT = 1 << 6;
        /// error if CREATE and the file exists
        const O_EXCL = 1 << 7;
        /// not become the process's controlling terminal
        const O_NOCTTY = 1 << 8;
        /// truncate file upon open
        const O_TRUNC = 1 << 9;
        /// file is a directory
        const O_DIRECTORY = O_DIRECTORY_BITS;
        /// pathname is not a symbolic link
        const O_NOFOLLOW = O_NOFOLLOW_BITS;
        /// close on exec
        const O_CLOEXEC = 1 << 19;
        /// create an unnamed temporary file
        const O_TMPFILE = 1 << 22;
    }
}
