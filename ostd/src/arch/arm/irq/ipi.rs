// SPDX-License-Identifier: MPL-2.0

//! Inter-processor interrupts.

use core::arch::asm;

use spin::Once;

use super::{IRQ_CHIP, MappedIrqLine};
use crate::{cpu::PinCurrentCpu, irq::IrqLine};

pub(super) const IPI_SGI_ID: u8 = 0;

static IPI_IRQ: Once<MappedIrqLine> = Once::new();

/// Hardware-specific, architecture-dependent CPU ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HwCpuId(u64);

impl HwCpuId {
    pub(crate) fn read_current(_guard: &dyn PinCurrentCpu) -> Self {
        Self(crate::arch::boot::smp::current_hw_cpu_id())
    }
}

/// Initializes the inter-processor interrupt on the BSP.
pub(in crate::arch) fn init_on_bsp() {
    let irq_line = IrqLine::alloc().expect("failed to allocate an IRQ line for AArch64 IPIs");
    let mut mapped_irq_line = IRQ_CHIP
        .get()
        .expect("GIC must be initialized before IPIs")
        .map_ipi_to(irq_line)
        .expect("failed to map the AArch64 IPI SGI");
    mapped_irq_line.on_active(|trapframe| {
        // SAFETY: This callback is registered only on the dedicated IPI SGI.
        unsafe { crate::smp::do_inter_processor_call(trapframe) }
    });
    IPI_IRQ.call_once(|| mapped_irq_line);
}

/// Checks that the BSP initialized the shared IPI mapping before this AP started.
pub(in crate::arch) fn init_on_ap() {
    assert!(IPI_IRQ.is_completed());
}

/// Sends a general inter-processor interrupt (IPI) to the specified CPU.
pub(crate) fn send_ipi(hw_cpu_id: HwCpuId, _guard: &dyn PinCurrentCpu) {
    let sgi1r = encode_sgi1r(hw_cpu_id.0, IPI_SGI_ID);
    // SAFETY: ICC_SGI1R_EL1 is the architectural interface for generating a Group-1 SGI. The
    // target affinity comes from an online CPU that was reported during SMP boot.
    unsafe {
        asm!(
            "dsb ishst",
            "msr icc_sgi1r_el1, {sgi1r}",
            sgi1r = in(reg) sgi1r,
            options(nostack, preserves_flags),
        )
    }
}

const fn encode_sgi1r(mpidr: u64, intid: u8) -> u64 {
    let aff0 = mpidr & 0xff;
    let aff1 = (mpidr >> 8) & 0xff;
    let aff2 = (mpidr >> 16) & 0xff;
    let aff3 = (mpidr >> 32) & 0xff;
    let range_selector = aff0 >> 4;
    let target_list = 1u64 << (aff0 & 0xf);

    (aff3 << 48)
        | (range_selector << 44)
        | (aff2 << 32)
        | ((intid as u64) << 24)
        | (aff1 << 16)
        | target_list
}

#[cfg(ktest)]
mod test {
    use crate::prelude::ktest;

    #[ktest]
    fn encodes_all_sgi_affinity_fields() {
        let mpidr = 0x12_0034_5678;
        let encoded = super::encode_sgi1r(mpidr, 7);
        assert_eq!(encoded, 0x0012_7034_0756_0100);
    }
}
