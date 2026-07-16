// SPDX-License-Identifier: MPL-2.0

use ostd::mm::VmIo;

use super::SyscallReturn;
use crate::{
    prelude::*,
    process::{
        Pid,
        signal::{
            c_types::siginfo_t,
            constants::SI_TKILL,
            sig_num::SigNum,
            signals::{Signal, raw::RawSignal},
        },
        tgkill,
    },
    thread::Tid,
};

/// Sends a queued signal with caller-supplied `siginfo_t` to a specific thread.
///
/// Linux permits non-negative `si_code` values only when a thread sends the
/// signal to itself. Crash handlers rely on that exception to re-raise a
/// synchronous hardware fault without losing its original `siginfo_t`.
pub fn sys_rt_tgsigqueueinfo(
    tgid: Pid,
    tid: Tid,
    sig_num: u8,
    info_ptr: Vaddr,
    ctx: &Context,
) -> Result<SyscallReturn> {
    if tgid.cast_signed() <= 0 || tid.cast_signed() <= 0 {
        return_errno_with_message!(Errno::EINVAL, "non-positive TGIDs or TIDs are not valid");
    }

    let signal_num = if sig_num == 0 {
        None
    } else {
        Some(SigNum::try_from(sig_num)?)
    };
    let mut siginfo = ctx.user_space().read_val::<siginfo_t>(info_ptr)?;
    // Linux treats the syscall argument as authoritative and overwrites the
    // user-provided field while copying the structure into the kernel.
    siginfo.si_signo = i32::from(sig_num);

    if tid != ctx.posix_thread.tid() && (siginfo.si_code >= 0 || siginfo.si_code == SI_TKILL) {
        return_errno_with_message!(
            Errno::EPERM,
            "custom signal information can only be sent to the calling thread"
        );
    }

    let signal = signal_num.map(|_| Box::new(RawSignal::new(siginfo)) as Box<dyn Signal>);
    tgkill(tid, Some(tgid), signal, ctx)?;
    Ok(SyscallReturn::Return(0))
}
