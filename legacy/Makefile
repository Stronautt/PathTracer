# **************************************************************************** #
#                                                                              #
#                                                                              #
#    Makefile                                                                  #
#                                                                              #
#    By: stronautt                                                             #
#                                                                              #
#    Created: 2023/08/07 14:53:53 by stronautt                                 #
#    Updated: 2023/08/07 16:15:14 by stronautt                                 #
#                                                                              #
# **************************************************************************** #

CC = gcc
DEFAULT_INTERPREU = /bin/bash
BUILD_DIR = ./build
BUILD_DEPENDENCIES = ${BUILD_DIR} ${BUILD_DIR}/Makefile

all: ${BUILD_DEPENDENCIES}
	@+${DEFAULT_INTERPREU} -c "make -s -C ${BUILD_DIR}";

${BUILD_DEPENDENCIES}:
	@${DEFAULT_INTERPREU} -c \
		"mkdir -p ${BUILD_DIR} && \
		cd ${BUILD_DIR} && \
		CC=${CC} cmake ..";

clean: ${BUILD_DEPENDENCIES}
	@${DEFAULT_INTERPREU} -c \
		"cd ${BUILD_DIR} && \
		find . -name \"*.o\" -delete";

fclean: ${BUILD_DEPENDENCIES}
	@+${DEFAULT_INTERPREU} -c \
		"cd ${BUILD_DIR} && \
		make -s clean";

re: fclean all

.NOTPARALLEL:
