// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include <signal.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "../../common/test.h"

static volatile sig_atomic_t deliveries;
static volatile sig_atomic_t delivered_signal;
static volatile sig_atomic_t delivered_signo;
static volatile sig_atomic_t delivered_code;
static volatile sig_atomic_t delivered_value;

static long rt_tgsigqueueinfo(pid_t tgid, pid_t tid, int sig, siginfo_t *info)
{
	return syscall(SYS_rt_tgsigqueueinfo, tgid, tid, sig, info);
}

static void handle_signal(int sig, siginfo_t *info, void *context)
{
	(void)context;
	delivered_signal = sig;
	delivered_signo = info->si_signo;
	delivered_code = info->si_code;
	delivered_value = info->si_value.sival_int;
	deliveries++;
}

static void init_siginfo(siginfo_t *info, int sig, int code)
{
	memset(info, 0, sizeof(*info));
	info->si_signo = sig;
	info->si_code = code;
	info->si_pid = getpid();
	info->si_uid = getuid();
}

FN_SETUP(install_rt_tgsigqueueinfo_handler)
{
	struct sigaction action = {
		.sa_sigaction = handle_signal,
		.sa_flags = SA_SIGINFO,
	};

	CHECK(sigemptyset(&action.sa_mask));
	CHECK(sigaction(SIGUSR1, &action, NULL));
}
END_SETUP()

FN_TEST(rt_tgsigqueueinfo_errnos)
{
	pid_t tgid = getpid();
	pid_t tid = syscall(SYS_gettid);
	siginfo_t info;

	init_siginfo(&info, SIGUSR1, SI_QUEUE);
	TEST_ERRNO(rt_tgsigqueueinfo(0, tid, SIGUSR1, &info), EINVAL);
	TEST_ERRNO(rt_tgsigqueueinfo(tgid, 0, SIGUSR1, &info), EINVAL);
	TEST_ERRNO(rt_tgsigqueueinfo(tgid + 1, tid, SIGUSR1, &info), ESRCH);
	TEST_ERRNO(rt_tgsigqueueinfo(tgid, tid, SIGUSR1, (siginfo_t *)1),
		   EFAULT);

	init_siginfo(&info, 65, SI_QUEUE);
	TEST_ERRNO(rt_tgsigqueueinfo(tgid, tid, 65, &info), EINVAL);

	init_siginfo(&info, SIGUSR1, SI_KERNEL);
	TEST_ERRNO(rt_tgsigqueueinfo(tgid, tid + 1, SIGUSR1, &info), EPERM);
}
END_TEST()

FN_TEST(rt_tgsigqueueinfo_uses_the_syscall_signal_number)
{
	pid_t tgid = getpid();
	pid_t tid = syscall(SYS_gettid);
	siginfo_t info;

	deliveries = 0;
	init_siginfo(&info, SIGUSR2, SI_QUEUE);
	TEST_SUCC(rt_tgsigqueueinfo(tgid, tid, SIGUSR1, &info));
	TEST_RES(deliveries, _ret == 1);
	TEST_RES(delivered_signal, _ret == SIGUSR1);
	TEST_RES(delivered_signo, _ret == SIGUSR1);
}
END_TEST()

FN_TEST(rt_tgsigqueueinfo_preserves_queued_info)
{
	pid_t tgid = getpid();
	pid_t tid = syscall(SYS_gettid);
	siginfo_t info;

	deliveries = 0;
	init_siginfo(&info, SIGUSR1, SI_QUEUE);
	info.si_value.sival_int = 0x12345678;
	TEST_SUCC(rt_tgsigqueueinfo(tgid, tid, SIGUSR1, &info));
	TEST_RES(deliveries, _ret == 1);
	TEST_RES(delivered_code, _ret == SI_QUEUE);
	TEST_RES(delivered_value, _ret == 0x12345678);
}
END_TEST()

FN_TEST(rt_tgsigqueueinfo_allows_kernel_code_only_for_self)
{
	pid_t tgid = getpid();
	pid_t tid = syscall(SYS_gettid);
	siginfo_t info;

	deliveries = 0;
	init_siginfo(&info, SIGUSR1, SI_KERNEL);
	TEST_SUCC(rt_tgsigqueueinfo(tgid, tid, SIGUSR1, &info));
	TEST_RES(deliveries, _ret == 1);
	TEST_RES(delivered_code, _ret == SI_KERNEL);
}
END_TEST()

FN_TEST(rt_tgsigqueueinfo_null_signal_is_an_existence_probe)
{
	pid_t tgid = getpid();
	pid_t tid = syscall(SYS_gettid);
	siginfo_t info;

	deliveries = 0;
	init_siginfo(&info, 0, SI_QUEUE);
	TEST_SUCC(rt_tgsigqueueinfo(tgid, tid, 0, &info));
	TEST_RES(deliveries, _ret == 0);
}
END_TEST()
