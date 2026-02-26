# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PathTracer is a GPU-accelerated 3D PBR path tracer using wgpu (WebGPU compute). It renders scenes with geometric primitives, fractals (Mandelbulb, Julia set), and imported OBJ models. Written in Rust (rewrite of the original C/OpenCL implementation).

## Build & Run

```bash
cargo build             # Debug build
cargo build --release   # Release build (optimized)
cargo run -- resources/scenes/demo.yaml   # Run with a scene file
```

**Requirements:** Vulkan/Metal/DX12-capable GPU (wgpu backend), Rust 2024 edition.

## Architecture

### Rendering Pipeline
Entry point `src/main.rs` → `app::run()`. The main loop: wgpu compute dispatch (path tracing) → progressive accumulation → post-processing → egui UI overlay → winit display → input handling → camera updates.

### Key Modules
- **`src/app.rs`** — App shell (winit `ApplicationHandler`, event dispatch). Delegates to submodules:
  - **`src/app/state.rs`** — `AppState` struct, initialization, GPU resource management, bind group helpers
  - **`src/app/rendering.rs`** — Per-frame update/render loop, UI action handling, screenshots
  - **`src/app/scene_ops.rs`** — Scene loading/saving, shape add/delete, OBJ model import
  - **`src/app/interaction.rs`** — Object picking, mouse drag, window/keyboard event routing
- **`src/constants.rs`** — Centralized numeric constants (GPU workgroup size, BVH params, camera defaults, window size, paths)
- **`src/scene/shape.rs`** — Shape types (17 geometric primitives), GPU representation
- **`src/scene/material.rs`** — PBR material (Cook-Torrance/GGX): base_color, metallic, roughness, transmission, IOR, emission
- **`src/scene/loader.rs`** / **`exporter.rs`** — YAML scene loading/saving (JSON backward-compatible for `.json` files)
- **`src/gpu/`** — wgpu context, buffer management, compute/render pipeline
- **`src/render/`** — Frame dispatch, progressive sample accumulator, post-processing effects
- **`src/shaders/composer.rs`** — WGSL shader composition (`// #import` preprocessor)
- **`src/ui/toolbar.rs`** — Top menu bar (Scene, Settings, Add Shape, shape list with grouping)
- **`src/ui/object_editor.rs`** — Right panel for editing selected object properties
- **`src/camera/`** — Camera projection and input controller
- **`src/accel/`** — BVH acceleration structure with AABBs
- **`src/model/obj_loader.rs`** — Wavefront OBJ loader (via tobj), MTL material mapping
- **`src/picking.rs`** — Ray-casting for object selection/dragging (BVH-accelerated)
- **`src/input/handler.rs`** — Keyboard/mouse input handling
- **`src/io/`** — Screenshot (PNG) and texture atlas

### Scene Format
Scenes are YAML files (`.yaml`) in `resources/scenes/`. JSON (`.json`) is also supported for backward compatibility (detected by file extension). They define camera (position, rotation, FOV, exposure), shapes (type, position, material, geometry params), and optional OBJ model paths. Scene export always uses YAML with flow-style numeric arrays.

### Shape Types
- **Elementary:** Sphere, Plane, Cube, Cylinder, Cone, Disc, Triangle, Pyramid, Tetrahedron
- **Complex:** Torus, Ellipsoid, Paraboloid, Hyperboloid, Mebius, Mandelbulb, Julia, Skybox

### Dependencies
See `Cargo.toml`. Key crates: wgpu (GPU), winit (windowing), egui (UI), glam (math), serde/serde_yml/serde_json (serialization), tobj (OBJ loading), bytemuck (GPU struct mapping), rfd (native file dialogs), image (texture/icon loading).

## Legacy Code

The original C/OpenCL implementation is preserved in the `legacy/` directory for reference:
- **`legacy/src/`** — C source files (scene parsing, OBJ loading, figure creation, UI design, OpenCL rendering)
- **`legacy/include/`** — C headers (`rt.h`, `structures.h`, `render.h.cl`)
- **`legacy/resources/kernels/`** — OpenCL kernels (path tracing, fractals, math utilities)
- **`legacy/dep/`** — Git submodules (libft, sgl)
- **`legacy/resources/scenes/`** — Original `.sc` scene files and converter script
- **`legacy/resources/fonts/`** — SDL2_ttf fonts (PasseroOne, SourceSans)
- **`legacy/CMakeLists.txt`** / **`legacy/Makefile`** — Legacy CMake build system
