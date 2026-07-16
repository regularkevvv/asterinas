// SPDX-License-Identifier: MPL-2.0

//! Power management.
//!
use crate::power::{ExitCode, inject_poweroff_handler, inject_restart_handler};

fn try_poweroff(_code: ExitCode) {
    super::psci::system_off();
}

fn try_restart(_code: ExitCode) {
    super::psci::system_reset();
}

pub(super) fn init() {
    if !super::psci::is_available() {
        return;
    }

    inject_poweroff_handler(try_poweroff);
    inject_restart_handler(try_restart);
}
