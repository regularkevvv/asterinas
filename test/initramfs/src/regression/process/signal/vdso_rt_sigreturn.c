// SPDX-License-Identifier: MPL-2.0

// Verify AArch64's kernel-provided signal restorer when an application does
// not provide SA_RESTORER. Go uses this rt_sigaction form on AArch64.
#define _GNU_SOURCE

#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/syscall.h>
#include <unistd.h>

struct kernel_sigaction {
	void (*handler)(int);
	unsigned long flags;
	void (*restorer)(void);
	uint64_t mask;
};

static volatile sig_atomic_t signal_seen;

static void handle_sigusr1(int signum)
{
	if (signum == SIGUSR1)
		signal_seen = 1;
}

int main(void)
{
#if !defined(__aarch64__)
	puts("SKIP: AArch64 vDSO signal-return regression");
	return EXIT_SUCCESS;
#else
	const struct kernel_sigaction action = {
		.handler = handle_sigusr1,
		.flags = SA_RESTART,
		.restorer = NULL,
		.mask = 0,
	};

	if (syscall(SYS_rt_sigaction, SIGUSR1, &action, NULL, sizeof(action.mask)) != 0) {
		perror("rt_sigaction");
		return EXIT_FAILURE;
	}
	if (kill(getpid(), SIGUSR1) != 0) {
		perror("kill");
		return EXIT_FAILURE;
	}
	if (!signal_seen) {
		fputs("SIGUSR1 handler did not return through the vDSO\n", stderr);
		return EXIT_FAILURE;
	}

	puts("AArch64 vDSO rt_sigreturn fallback works");
	return EXIT_SUCCESS;
#endif
}
