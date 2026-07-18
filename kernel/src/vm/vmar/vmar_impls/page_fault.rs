// SPDX-License-Identifier: MPL-2.0

use super::{Interval, RssDelta, Vmar};
use crate::{prelude::*, vm::perms::VmPerms};

impl Vmar {
    pub fn handle_page_fault(&self, page_fault_info: &PageFaultInfo) -> Result<()> {
        // Kernel accesses to userspace can fault while RCU or another
        // preemption guard is held. A contended sleeping read would try to
        // reschedule from atomic mode and panic, so retain the existing
        // sleeping behavior for ordinary faults and use the explicitly
        // non-sleeping acquisition only for atomic-mode faults.
        let inner = if ostd::task::atomic_mode::is_atomic() {
            self.inner.spin_read()
        } else {
            self.inner.read()
        };

        let address = page_fault_info.address;
        if let Some(vm_mapping) = inner.vm_mappings.find_one(&address) {
            debug_assert!(vm_mapping.range().contains(&address));

            let mut rss_delta = RssDelta::new(self);
            return vm_mapping.handle_page_fault(&self.vm_space, page_fault_info, &mut rss_delta);
        }

        return_errno_with_message!(
            Errno::EACCES,
            "no VM mappings contain the page fault address"
        );
    }
}

/// Page fault information converted from [`CpuException`].
///
/// `TryFrom<CpuException>` should be implemented for this struct.
/// If [`CpuException`] is a page fault, `try_from` should return `Ok(PageFaultInfo)`,
/// or `Err(())` (no error information) otherwise.
///
/// [`CpuException`]: ostd::arch::cpu::context::CpuException
#[derive(Debug)]
pub struct PageFaultInfo {
    /// The virtual address where a page fault occurred.
    pub(in crate::vm::vmar) address: Vaddr,

    /// The [`VmPerms`] required by the memory operation that causes page fault.
    /// For example, a "store" operation may require `VmPerms::WRITE`.
    pub(in crate::vm::vmar) required_perms: VmPerms,

    /// Whether this page fault is forced (e.g., manually triggered by `ptrace`).
    /// A forced page fault may bypass some permission checks.
    is_forced: bool,
}

impl PageFaultInfo {
    /// Creates a new `PageFaultInfo`.
    pub fn new(address: Vaddr, required_perms: VmPerms) -> Self {
        Self {
            address,
            required_perms,
            is_forced: false,
        }
    }

    /// Returns whether this page fault is forced.
    pub(in crate::vm::vmar) fn is_forced(&self) -> bool {
        self.is_forced
    }

    /// Marks this page fault as forced.
    pub(super) fn force(mut self) -> Self {
        self.is_forced = true;
        self
    }
}
