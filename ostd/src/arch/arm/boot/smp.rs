// SPDX-License-Identifier: MPL-2.0

//! Multiprocessor Boot Support

use core::arch::{asm, global_asm};

use crate::{boot::smp::PerApRawInfo, mm::Paddr};

global_asm!(include_str!("ap_boot.S"));

const MPIDR_AFFINITY_MASK: u64 = 0xff00_ffffff;

pub(crate) fn count_processors() -> Option<u32> {
    let bsp_id = current_hw_cpu_id();
    let mut count = 0u32;
    let mut found_bsp = false;

    for_each_supported_cpu(|hw_cpu_id| {
        count = count
            .checked_add(1)
            .expect("too many CPUs in the device tree");
        found_bsp |= hw_cpu_id == bsp_id;
    });

    (count > 0 && found_bsp).then_some(count)
}

/// Brings up all application processors.
///
/// Following the x86 naming, all processors other than the bootstrap processor
/// are "application processors".
///
/// # Safety
///
/// The caller must ensure that
///  1. we're in the boot context of the BSP,
///  2. all APs have not yet been booted, and
///  3. the arguments are valid to boot APs.
pub(crate) unsafe fn bringup_all_aps(info_ptr: *const PerApRawInfo, pt_ptr: Paddr, num_cpus: u32) {
    if num_cpus <= 1 {
        return;
    }

    assert!(
        super::super::psci::is_available(),
        "PSCI is required to boot AArch64 APs"
    );

    // SAFETY: These symbols are writable pointer slots in the AP boot assembly. APs have not been
    // started yet, so the BSP has exclusive access to them. Symbols in `.ap_boot` have physical
    // linker addresses, so the BSP must add the kernel load offset before accessing them after the
    // final kernel page table has been activated.
    unsafe {
        let kernel_offset = crate::mm::kspace::kernel_loaded_offset();
        let info_slot = (core::ptr::addr_of_mut!(__ap_boot_info_array_pointer) as usize
            + kernel_offset) as *mut *const PerApRawInfo;
        let page_table_slot = (core::ptr::addr_of_mut!(__ap_boot_page_table_pointer) as usize
            + kernel_offset) as *mut Paddr;
        info_slot.write(info_ptr);
        page_table_slot.write(pt_ptr);
        asm!("dsb ishst", options(nostack, preserves_flags));
    }

    let bsp_id = current_hw_cpu_id();
    let entry_point = ap_boot_start as *const () as Paddr;
    let mut next_cpu_id = 1u32;

    for_each_supported_cpu(|hw_cpu_id| {
        if hw_cpu_id == bsp_id {
            return;
        }

        let cpu_id = next_cpu_id;
        next_cpu_id = next_cpu_id
            .checked_add(1)
            .expect("the number of AArch64 CPUs exceeds u32::MAX");
        crate::info!("Starting processor {} (MPIDR {:#x})", cpu_id, hw_cpu_id);

        if let Err(error) = super::super::psci::cpu_on(hw_cpu_id, entry_point, u64::from(cpu_id)) {
            panic!(
                "Failed to start processor {} (MPIDR {:#x}) using PSCI: error {}",
                cpu_id, hw_cpu_id, error
            );
        }
    });

    assert_eq!(
        next_cpu_id, num_cpus,
        "device-tree CPU count changed during boot"
    );
}

fn for_each_supported_cpu(mut f: impl FnMut(u64)) {
    let Some(device_tree) = super::DEVICE_TREE.get() else {
        f(current_hw_cpu_id());
        return;
    };

    for cpu in device_tree.cpus() {
        if cpu
            .property("status")
            .and_then(|status| status.as_str())
            .is_some_and(|status| status != "ok" && status != "okay")
        {
            continue;
        }

        let hw_cpu_id = normalize_mpidr(cpu.ids().first() as u64);
        if hw_cpu_id != current_hw_cpu_id()
            && cpu
                .property("enable-method")
                .and_then(|method| method.as_str())
                != Some("psci")
        {
            continue;
        }
        f(hw_cpu_id);
    }
}

pub(in crate::arch) fn current_hw_cpu_id() -> u64 {
    let mpidr: u64;
    // SAFETY: Reading MPIDR_EL1 has no side effects.
    unsafe {
        asm!("mrs {mpidr}, mpidr_el1", mpidr = out(reg) mpidr, options(nomem, nostack, preserves_flags))
    };
    normalize_mpidr(mpidr)
}

const fn normalize_mpidr(mpidr: u64) -> u64 {
    mpidr & MPIDR_AFFINITY_MASK
}

unsafe extern "C" {
    fn ap_boot_start();
    static mut __ap_boot_info_array_pointer: *const PerApRawInfo;
    static mut __ap_boot_page_table_pointer: Paddr;
}
