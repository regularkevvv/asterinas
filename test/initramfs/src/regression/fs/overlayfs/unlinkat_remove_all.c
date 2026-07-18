// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE
#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "../../common/test.h"

#define PARENT_DIR "/tmp"
#define TEST_DIR "asterinas-unlinkat-remove-all"
#define CHILD_DIR "child"
#define TEST_FILE "file"

static void cleanup_test_tree(void)
{
	unlink(PARENT_DIR "/" TEST_DIR "/" CHILD_DIR "/" TEST_FILE);
	rmdir(PARENT_DIR "/" TEST_DIR "/" CHILD_DIR);
	rmdir(PARENT_DIR "/" TEST_DIR);
}

static void remove_files_then_directory(int parent_fd, const char *name)
{
	int fd = CHECK(openat(parent_fd, name, O_RDONLY | O_DIRECTORY));
	DIR *dir = fdopendir(fd);
	CHECK(dir != NULL);

	for (struct dirent *entry = readdir(dir); entry != NULL;
	     entry = readdir(dir)) {
		if (entry->d_name[0] == '.'
		    && (entry->d_name[1] == '\0'
			|| (entry->d_name[1] == '.' && entry->d_name[2] == '\0')))
			continue;

		CHECK(unlinkat(fd, entry->d_name, 0));
	}

	CHECK(closedir(dir));
	CHECK(unlinkat(parent_fd, name, AT_REMOVEDIR));
}

static void remove_subdirectories_from_directory(int fd)
{
	DIR *dir = fdopendir(dup(fd));
	CHECK(dir != NULL);

	for (struct dirent *entry = readdir(dir); entry != NULL;
	     entry = readdir(dir)) {
		if (entry->d_name[0] == '.'
		    && (entry->d_name[1] == '\0'
			|| (entry->d_name[1] == '.' && entry->d_name[2] == '\0')))
			continue;

		remove_files_then_directory(fd, entry->d_name);
	}

	CHECK(closedir(dir));
}

/*
 * Go's os.RemoveAll walks a directory with openat/readdir and then removes
 * each child and the parent with unlinkat(AT_REMOVEDIR). This must work on
 * the writable root overlay too: Go uses it to clean its race-test work dir.
 */
FN_TEST(unlinkat_remove_all_after_directory_walk)
{
	cleanup_test_tree();

	int parent_fd = TEST_SUCC(open(PARENT_DIR, O_RDONLY | O_DIRECTORY));
	TEST_SUCC(mkdirat(parent_fd, TEST_DIR, 0700));

	int test_fd =
		TEST_SUCC(openat(parent_fd, TEST_DIR, O_RDONLY | O_DIRECTORY));
	TEST_SUCC(mkdirat(test_fd, CHILD_DIR, 0700));

	int child_fd =
		TEST_SUCC(openat(test_fd, CHILD_DIR, O_RDONLY | O_DIRECTORY));
	int file_fd = TEST_SUCC(
		openat(child_fd, TEST_FILE, O_CREAT | O_WRONLY, 0600));
	TEST_SUCC(close(file_fd));

	TEST_SUCC(close(child_fd));

	/*
	 * Keep this directory descriptor open while walking it, as Go's
	 * os.RemoveAll does. The walker must observe and remove CHILD_DIR.
	 */
	remove_subdirectories_from_directory(test_fd);
	TEST_SUCC(close(test_fd));
	TEST_SUCC(unlinkat(parent_fd, TEST_DIR, AT_REMOVEDIR));

	TEST_SUCC(close(parent_fd));

	cleanup_test_tree();
}
END_TEST()
