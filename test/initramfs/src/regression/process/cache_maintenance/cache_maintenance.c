// SPDX-License-Identifier: MPL-2.0

#include <stdint.h>
#include <string.h>

#include "../../common/test.h"

#if defined(__aarch64__)
static int exercise_el0_cache_controls(void)
{
	unsigned long ctr;
	unsigned long dczid;
	unsigned char zero_line[256] __attribute__((aligned(256)));

	asm volatile("mrs %0, ctr_el0" : "=r"(ctr));
	if (ctr == 0)
		return -1;

	asm volatile("dc cvau, %0\n"
		     "dsb ish\n"
		     "ic ivau, %0\n"
		     "dsb ish\n"
		     "isb\n"
		     :
		     : "r"(zero_line)
		     : "memory");

	asm volatile("mrs %0, dczid_el0" : "=r"(dczid));
	if ((dczid & (1UL << 4)) == 0) {
		size_t block_size = 4UL << (dczid & 0xf);

		if (block_size > sizeof(zero_line))
			return -1;
		memset(zero_line, 0xff, sizeof(zero_line));
		asm volatile("dc zva, %0" : : "r"(zero_line) : "memory");
		for (size_t i = 0; i < block_size; i++) {
			if (zero_line[i] != 0)
				return -1;
		}
	}

	return 0;
}
#endif

FN_TEST(aarch64_el0_cache_controls)
{
#if defined(__aarch64__)
	TEST_RES(exercise_el0_cache_controls(), _ret == 0);
#else
	SKIP_TEST_IF(1);
#endif
}
END_TEST()
