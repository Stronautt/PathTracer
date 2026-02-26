# **************************************************************************** #
#                                                                              #
#                                                                              #
#    Makefile                                                                  #
#                                                                              #
#    By: phrytsenko                                                            #
#                                                                              #
#    Created: 2019/01/05 14:53:53 by cozzmonavt                                #
#    Updated: 2019/01/08 16:15:14 by phrytsenko                                #
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
