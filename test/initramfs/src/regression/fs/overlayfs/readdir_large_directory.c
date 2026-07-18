// SPDX-License-Identifier: MPL-2.0

#include <fcntl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "../../common/test.h"

#define PARENT_DIR "/tmp"
#define TEST_DIR "asterinas-readdir-large"
#define ENTRY_COUNT 420
#define READ_BUFFER_SIZE 8192

struct linux_dirent64 {
	uint64_t d_ino;
	int64_t d_off;
	unsigned short d_reclen;
	unsigned char d_type;
	char d_name[];
};

static void entry_name(char *buffer, size_t buffer_len, int index)
{
	snprintf(buffer, buffer_len, "entry-%03d", index);
}

static void cleanup_test_directory(void)
{
	char name[16];
	char path[64];
	for (int index = 0; index < ENTRY_COUNT; index++) {
		entry_name(name, sizeof(name), index);
		snprintf(path, sizeof(path), PARENT_DIR "/" TEST_DIR "/%s", name);
		rmdir(path);
	}
	rmdir(PARENT_DIR "/" TEST_DIR);
}

FN_TEST(readdir_large_directory_does_not_skip_entries)
{
	cleanup_test_directory();

	int parent_fd = TEST_SUCC(open(PARENT_DIR, O_RDONLY | O_DIRECTORY));
	TEST_SUCC(mkdirat(parent_fd, TEST_DIR, 0700));
	int dir_fd = TEST_SUCC(openat(parent_fd, TEST_DIR, O_RDONLY | O_DIRECTORY));

	char name[16];
	for (int index = 0; index < ENTRY_COUNT; index++) {
		entry_name(name, sizeof(name), index);
		TEST_SUCC(mkdirat(dir_fd, name, 0700));
	}

	bool seen[ENTRY_COUNT] = { false };
	char buffer[READ_BUFFER_SIZE];
	for (;;) {
		long read_size = TEST_RES(
			syscall(SYS_getdents64, dir_fd, buffer, sizeof(buffer)),
			_ret >= 0);
		if (read_size == 0)
			break;

		for (long offset = 0; offset < read_size;) {
			struct linux_dirent64 *entry =
				(struct linux_dirent64 *)(buffer + offset);
			if (entry->d_reclen == 0) {
				fprintf(stderr, "zero-length directory entry\n");
				exit(EXIT_FAILURE);
			}
			int entry_index;
			if (sscanf(entry->d_name, "entry-%d", &entry_index) == 1
			    && entry_index >= 0 && entry_index < ENTRY_COUNT) {
				seen[entry_index] = true;
			}
			offset = (long)((char *)entry + entry->d_reclen - buffer);
		}
	}

	int missing = 0;
	for (int index = 0; index < ENTRY_COUNT; index++) {
		if (!seen[index]) {
			fprintf(stderr, "missing directory entry: entry-%03d\n", index);
			missing++;
		}
	}
	TEST_RES(missing, _ret == 0);

	TEST_SUCC(close(dir_fd));
	TEST_SUCC(close(parent_fd));
	cleanup_test_directory();
}
END_TEST()
