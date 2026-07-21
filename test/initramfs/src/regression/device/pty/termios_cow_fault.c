// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE
#include "../../common/test.h"

#include <pty.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>

FN_TEST(termios_copy_to_cow_page)
{
	int master;
	int slave;
	struct termios *termios = TEST_SUCC(mmap(NULL, 4096,
						 PROT_READ | PROT_WRITE,
						 MAP_PRIVATE | MAP_ANONYMOUS, -1, 0));
	TEST_SUCC(openpty(&master, &slave, NULL, NULL, NULL));

	// Populate the page before fork so the child's first kernel write faults
	// through a private COW mapping. TCGETS must perform that user copy after
	// releasing the IRQ-disabling line-discipline lock.
	termios->c_iflag = 0;
	pid_t child = TEST_SUCC(fork());
	if (child == 0) {
		if (ioctl(slave, TCGETS, termios) < 0)
			_exit(EXIT_FAILURE);
		_exit(EXIT_SUCCESS);
	}

	int status;
	TEST_RES(waitpid(child, &status, 0),
		 _ret == child && WIFEXITED(status) && WEXITSTATUS(status) == 0);
	TEST_SUCC(close(master));
	TEST_SUCC(close(slave));
	TEST_SUCC(munmap(termios, 4096));
}
END_TEST()
