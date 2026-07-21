// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

static atomic_int target_tid = ATOMIC_VAR_INIT(-1);
static atomic_bool target_ready = ATOMIC_VAR_INIT(false);
static atomic_bool signal_delivered = ATOMIC_VAR_INIT(false);
static int target_cpu;

static void pin_current_thread_to_cpu(int cpu)
{
	cpu_set_t cpuset;
	CPU_ZERO(&cpuset);
	CPU_SET(cpu, &cpuset);
	if (sched_setaffinity(0, sizeof(cpuset), &cpuset) != 0) {
		perror("sched_setaffinity");
		exit(EXIT_FAILURE);
	}
}

static void on_sigurg(int signum)
{
	(void)signum;
	atomic_store_explicit(&signal_delivered, true, memory_order_release);
}

static void *busy_target(void *arg)
{
	(void)arg;
	pin_current_thread_to_cpu(target_cpu);

	atomic_store_explicit(&target_tid, (int)syscall(SYS_gettid), memory_order_relaxed);
	atomic_store_explicit(&target_ready, true, memory_order_release);

	while (!atomic_load_explicit(&signal_delivered, memory_order_acquire)) {
		atomic_signal_fence(memory_order_seq_cst);
	}

	return NULL;
}

static long long monotonic_millis(void)
{
	struct timespec now;
	if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
		perror("clock_gettime");
		exit(EXIT_FAILURE);
	}
	return (long long)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

int main(void)
{
	long online_cpus = sysconf(_SC_NPROCESSORS_ONLN);
	if (online_cpus < 2) {
		printf("SKIP: remote tgkill delivery requires at least two CPUs\n");
		return EXIT_SUCCESS;
	}

	/* CPU 2 reproduces the third-CPU Go async-preemption boundary when present. */
	target_cpu = online_cpus > 2 ? 2 : 1;
	pin_current_thread_to_cpu(0);

	struct sigaction action = {
		.sa_handler = on_sigurg,
		.sa_flags = SA_RESTART,
	};
	sigemptyset(&action.sa_mask);
	if (sigaction(SIGURG, &action, NULL) != 0) {
		perror("sigaction");
		return EXIT_FAILURE;
	}

	pthread_t target;
	if (pthread_create(&target, NULL, busy_target, NULL) != 0) {
		perror("pthread_create");
		return EXIT_FAILURE;
	}

	long long ready_deadline = monotonic_millis() + 1000;
	while (!atomic_load_explicit(&target_ready, memory_order_acquire)) {
		if (monotonic_millis() > ready_deadline) {
			fprintf(stderr, "target thread did not start\n");
			return EXIT_FAILURE;
		}
	}

	int tid = atomic_load_explicit(&target_tid, memory_order_relaxed);
	if (syscall(SYS_tgkill, getpid(), tid, SIGURG) != 0) {
		perror("tgkill");
		return EXIT_FAILURE;
	}

	long long delivered_deadline = monotonic_millis() + 1000;
	while (!atomic_load_explicit(&signal_delivered, memory_order_acquire)) {
		if (monotonic_millis() > delivered_deadline) {
			fprintf(stderr, "SIGURG did not interrupt the running target thread\n");
			return EXIT_FAILURE;
		}
	}

	if (pthread_join(target, NULL) != 0) {
		perror("pthread_join");
		return EXIT_FAILURE;
	}

	printf("tgkill interrupted the running thread on CPU %d\n", target_cpu);
	return EXIT_SUCCESS;
}
