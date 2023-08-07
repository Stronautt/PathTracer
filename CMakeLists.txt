cmake_minimum_required(VERSION 3.5.2)

project(PathTracer LANGUAGES C)

find_package(OpenCL REQUIRED)
find_package(SDL2 REQUIRED)
find_package(SDL2_ttf REQUIRED)
find_package(SDL2_image REQUIRED)

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wextra -Wall -Werror")
set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -g -O0")
set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} -O3")

set(PROJECT_SRC_DIR "${PROJECT_SOURCE_DIR}/src")
set(PROJECT_HEADERS_DIR "${PROJECT_SOURCE_DIR}/include")

add_subdirectory(dep/libft)
add_subdirectory(dep/sgl)

add_executable(${PROJECT_NAME}
    src/main.c
    src/add_figure_1.c
    src/add_figure_2.c
    src/add_figure_3.c
    src/async_read.c
    src/events.c
    src/movement.c
    src/obj_info.c
    src/obj_parser_1.c
    src/obj_parser_2.c
    src/obj_parser_3.c
    src/obj_parser_4.c
    src/opencl.c
    src/render.c
    src/save.c
    src/scene_1.c
    src/scene_2.c
    src/scene_3.c
    src/scene_4.c
    src/stats.c
    src/sys_handler.c
    src/utils_1.c
    src/utils_2.c
    src/utils_3.c
    src/vec.c

    # Design Source Files
    src/design/design_1.c
    src/design/design_2.c
    src/design/design_3.c
    src/design/design_4.c
    src/design/design_5.c
    src/design/design_6.c
    src/design/design_7.c
    src/design/design_8.c
    src/design/design_9.c
    src/design/design_10.c
    src/design/design_11.c
    src/design/design_12.c
    src/design/design_13.c
    src/design/design_14.c
    src/design/design_15.c
    src/design/design_16.c
    src/design/design_17.c
    src/design/design_18.c
    src/design/design_19.c
    src/design/design_20.c
)
target_link_libraries(${PROJECT_NAME} PRIVATE OpenCL::OpenCL SDL2::SDL2 SDL2_ttf SDL2_image ft sgl)
target_include_directories(${PROJECT_NAME} PRIVATE ${PROJECT_HEADERS_DIR})
