// SPDX-License-Identifier: MPL-2.0

#include <errno.h>
#include <sys/socket.h>
#include <unistd.h>

#include "../common/test.h"

FN_TEST(blocking_socket_honors_msg_dontwait)
{
	int sockets[2];
	char byte;

	TEST_SUCC(socketpair(AF_UNIX, SOCK_STREAM, 0, sockets));
	errno = 0;
	TEST_RES(recv(sockets[0], &byte, sizeof(byte), MSG_DONTWAIT),
		 _ret == -1 && errno == EAGAIN);
	TEST_SUCC(close(sockets[0]));
	TEST_SUCC(close(sockets[1]));
}
END_TEST()
