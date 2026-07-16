// SPDX-License-Identifier: MPL-2.0

//! ARM Power State Coordination Interface (PSCI).
//!
//! Reference: <https://developer.arm.com/documentation/den0022/latest/>

use core::arch::asm;

use fdt::node::FdtNode;
use spin::Once;

use crate::{arch::boot::DEVICE_TREE, mm::Paddr};

#[derive(Clone, Copy, Debug)]
enum PsciMethod {
    Smc,
    Hvc,
}

impl PsciMethod {
    fn parse(node: &FdtNode) -> Option<Self> {
        match node.property("method")?.as_str()? {
            "smc" => Some(Self::Smc),
            "hvc" => Some(Self::Hvc),
            _ => None,
        }
    }
}

static PSCI_METHOD: Once<PsciMethod> = Once::new();

const PSCI_CPU_ON_64: u32 = 0xc400_0003;
const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;
const PSCI_SYSTEM_RESET: u32 = 0x8400_0009;

/// Initializes the PSCI conduit described by the device tree.
pub(super) fn init() {
    // Reference: <https://www.kernel.org/doc/Documentation/devicetree/bindings/arm/psci.txt>
    const FDT_COMPATIBLE: &[&str] = &["arm,psci-0.2", "arm,psci-1.0"];

    let device_tree = DEVICE_TREE.get().unwrap();
    let Some(psci_node) = device_tree.find_compatible(FDT_COMPATIBLE) else {
        crate::warn!("No PSCI 0.2+ node found in the device tree");
        return;
    };
    let Some(psci_method) = PsciMethod::parse(&psci_node) else {
        crate::warn!("PSCI node has an invalid method");
        return;
    };

    crate::info!("PSCI detected: {:?}", psci_method);
    PSCI_METHOD.call_once(|| psci_method);
}

pub(super) fn is_available() -> bool {
    PSCI_METHOD.is_completed()
}

/// Starts a powered-off CPU at `entry_point` and supplies `context_id` in `x0`.
pub(super) fn cpu_on(target_cpu: u64, entry_point: Paddr, context_id: u64) -> Result<(), i32> {
    let result = psci_call(PSCI_CPU_ON_64, target_cpu, entry_point as u64, context_id);
    if result == 0 { Ok(()) } else { Err(result) }
}

pub(super) fn system_off() {
    let _ = psci_call(PSCI_SYSTEM_OFF, 0, 0, 0);
}

pub(super) fn system_reset() {
    let _ = psci_call(PSCI_SYSTEM_RESET, 0, 0, 0);
}

fn psci_call(function_id: u32, arg0: u64, arg1: u64, arg2: u64) -> i32 {
    let Some(method) = PSCI_METHOD.get() else {
        return -1;
    };

    let mut result = function_id as u64;
    // SAFETY: The PSCI conduit and function IDs come from the PSCI specification, and the conduit
    // is invoked only after its presence has been established from the device tree.
    unsafe {
        match method {
            PsciMethod::Smc => asm!(
                "smc #0",
                inout("x0") result,
                inlateout("x1") arg0 => _,
                inlateout("x2") arg1 => _,
                inlateout("x3") arg2 => _,
                lateout("x4") _,
                lateout("x5") _,
                lateout("x6") _,
                lateout("x7") _,
                lateout("x8") _,
                lateout("x9") _,
                lateout("x10") _,
                lateout("x11") _,
                lateout("x12") _,
                lateout("x13") _,
                lateout("x14") _,
                lateout("x15") _,
                lateout("x16") _,
                lateout("x17") _,
                options(nostack),
            ),
            PsciMethod::Hvc => asm!(
                "hvc #0",
                inout("x0") result,
                inlateout("x1") arg0 => _,
                inlateout("x2") arg1 => _,
                inlateout("x3") arg2 => _,
                lateout("x4") _,
                lateout("x5") _,
                lateout("x6") _,
                lateout("x7") _,
                lateout("x8") _,
                lateout("x9") _,
                lateout("x10") _,
                lateout("x11") _,
                lateout("x12") _,
                lateout("x13") _,
                lateout("x14") _,
                lateout("x15") _,
                lateout("x16") _,
                lateout("x17") _,
                options(nostack),
            ),
        }
    }

    result as u32 as i32
}
