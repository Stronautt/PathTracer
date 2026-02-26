# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PathTracer is a GPU-accelerated 3D rendering application using the Path Tracing algorithm via OpenCL. It renders scenes with geometric primitives, fractals (Mandelbulb, Julia set), and imported OBJ models. Written in C.

## Build & Run

```bash
make              # Build (creates ./build/ directory, runs CMake + make)
make re           # Clean rebuild
make fclean       # Full clean (removes build artifacts)

./build/PathTracer resources/scenes/all.sc    # Run with a scene file
```

**Requirements:** OpenCL 1.2+, GPU device, SDL2, SDL2_ttf, SDL2_image.

## Architecture

### Rendering Pipeline
The main loop in `src/main.c` drives the application: GPU ray-tracing via OpenCL → post-processing effects → SDL2 display → event handling → camera movement updates. Rendering is done by OpenCL kernels in `resources/kernels/`, with `render.cl` being the main path-tracing kernel.

### Key Modules
- **`src/render.c`** / **`src/opencl.c`** — GPU rendering pipeline and OpenCL context/kernel management
- **`src/scene_1.c`–`scene_4.c`** — JSON scene file parser (`.sc` format)
- **`src/obj_parser_1.c`–`obj_parser_4.c`** — Wavefront OBJ model loader
- **`src/add_figure_1.c`–`add_figure_3.c`** — Primitive shape creation (16 types)
- **`src/events.c`** / **`src/movement.c`** — Input handling and camera controls
- **`src/async_read.c`** — Threaded scene and kernel loading with progress display
- **`src/design/design_1.c`–`design_20.c`** — UI control initialization (buttons, panels, textboxes)

### Headers
- **`include/rt.h`** — All function declarations, OpenCL struct definitions, enums
- **`include/structures.h`** — Core data structures (`t_obj`, `t_env`, `t_cam`, `t_scene`, etc.)

### Dependencies (git submodules in `dep/`)
- **`libft`** — Custom C standard library (static lib)
- **`sgl`** — SDL2 graphics abstraction layer (static lib, depends on SDL2/SDL2_ttf/SDL2_image)

### GPU Kernels (`resources/kernels/`)
- **`render.cl`** — Main path-tracing kernel (ray-object intersections, materials, light transport)
- **`julia.cl`** / **`mandelbulb.cl`** / **`mebius.cl`** — Fractal/surface algorithms
- **`complex.cl`** / **`solveP4.cl`** — Math utilities (complex arithmetic, quartic solver)

### Scene Format
Scenes are JSON files (`.sc` extension) in `resources/scenes/`. They define camera position/rotation, ambient light, figures (with type, position, material, color), and optional OBJ model paths.

### Build System
CMake 3.5.2+ with a Makefile wrapper. Compiler flags: `-Wextra -Wall -Wno-cast-function-type -Werror`. Links against: OpenCL, libft, sgl, math, pthreads. Platform-conditional OpenCL headers for Linux vs macOS.
