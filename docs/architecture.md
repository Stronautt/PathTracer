# Architecture

## Overview

PathTracer is a GPU-accelerated physically-based path tracer built with Rust and wgpu (WebGPU). It runs compute shaders on the GPU via Vulkan/Metal/DX12 backends.

## Module Map

```
src/
  main.rs           Entry point: CLI args, winit event loop
  constants.rs      Centralized numeric constants (GPU, BVH, camera, window defaults)

  app.rs            App shell: winit ApplicationHandler, event dispatch
  app/
    state.rs        AppState struct, initialization, GPU resource management, bind groups
    rendering.rs    Per-frame update loop, render dispatch, UI action handling, screenshots
    scene_ops.rs    Scene loading/saving, shape add/delete, OBJ model import
    interaction.rs  Object picking, mouse drag, window/keyboard event routing

  gpu/
    context.rs      wgpu instance/adapter/device/queue/surface
    pipeline.rs     Compute + render pipeline creation helpers
    buffers.rs      GPU buffer creation and update utilities

  render/
    frame.rs        Per-frame dispatch: trace -> post-process -> blit
    accumulator.rs  Progressive refinement sample counting and reset
    post_process.rs Post-processing effect enum (Negative, Sepia, FXAA, etc.)

  shaders/
    composer.rs     WGSL "// #import module" preprocessor (Bevy-style)
    wgsl/           All WGSL shader source files (see Shader Pipeline below)

  scene/
    scene.rs        Scene struct (camera config, shapes, model refs)
    loader.rs       YAML deserialization via serde_yml (JSON fallback for .json files)
    exporter.rs     YAML serialization (scene save)
    shape.rs        ShapeType enum (17 types), Shape struct, GpuShape
    material.rs     PBR Material struct, GpuMaterial

  accel/
    aabb.rs         Axis-aligned bounding box, per-shape AABB computation
    bvh.rs          SAH-based BVH construction, flat GPU node array

  model/
    obj_loader.rs   OBJ file loading via tobj -> triangle shapes, MTL material mapping

  camera/
    camera.rs       Camera state, quaternion orientation, GPU struct
    controller.rs   FPS-style WASD + mouse controller

  ui/
    mod.rs          egui integration, draw_ui entry point, modal dialogs
    toolbar.rs      Top menu bar (Scene, Settings, Add Shape), shape list with grouping
    object_editor.rs  Right panel: PBR material sliders, position editor, fractal params

  input/
    handler.rs      Keyboard/mouse event -> controller state flags

  io/
    screenshot.rs   Read GPU buffer -> save PNG
    texture_atlas.rs  Pack textures into flat GPU buffer

  picking.rs        Ray-casting for object selection/dragging (BVH-accelerated)
```

## App Module Split

The `app` module is split into focused submodules connected via `AppState`:

- **`app.rs`** (shell) -- Defines the winit `ApplicationHandler`, creates `AppState` on resume, delegates window/device events to submodules.
- **`app/state.rs`** -- `AppState` struct holding all GPU resources, scene data, camera, UI state. Initialization (`new()`), bind group layout/creation helpers, resize handling, scene buffer rebuilds.
- **`app/rendering.rs`** -- `update_and_render()`: per-frame camera update, egui UI pass, compute dispatch, blit, present. Also `apply_ui_actions()` and screenshot capture.
- **`app/scene_ops.rs`** -- `add_shape()`, `delete_shape()`, `save_scene()`, `import_scene()`, `import_model()`.
- **`app/interaction.rs`** -- `handle_window_event()`: keyboard/mouse routing, object picking on click, mouse-drag shape movement, focus-loss cleanup.

## Constants

`src/constants.rs` centralizes all numeric constants previously scattered as magic numbers:

| Constant | Value | Purpose |
|----------|-------|---------|
| `WORKGROUP_SIZE` | 8 | GPU compute workgroup dimensions (8x8) |
| `BVH_NUM_BINS` | 12 | SAH bin count for BVH split search |
| `BVH_LEAF_MAX_PRIMS` | 4 | Max primitives in a BVH leaf node |
| `AABB_EPS` | 0.0001 | Padding for degenerate AABBs |
| `DEFAULT_FOV` | 60.0 | Camera field of view (degrees) |
| `DEFAULT_EXPOSURE` | 1.0 | Camera exposure multiplier |
| `DEFAULT_CAMERA_POSITION` | [0, 2, -10] | Camera starting position |
| `CAMERA_DEFAULT_MOVE_SPEED` | 5.0 | Movement speed (units/sec) |
| `CAMERA_SPRINT_MULTIPLIER` | 3.0 | Sprint speed multiplier |
| `CAMERA_DEFAULT_SENSITIVITY` | 0.15 | Mouse look sensitivity |
| `MODEL_AUTO_SCALE_TARGET` | 3.0 | Auto-scale imported OBJ models to this size |
| `ACCUM_BYTES_PER_PIXEL` | 16 | vec4<f32> accumulation buffer stride |
| `DEFAULT_WINDOW_WIDTH/HEIGHT` | 1280x720 | Initial window dimensions |
| `DEFAULT_SCENE_PATH` | `resources/scenes/demo.yaml` | Fallback scene |
| `POST_PARAMS_SIZE` | 12 | Post-process uniform array size |
| `POST_PARAMS_MAX_EFFECTS` | 8 | Max stackable post-process effects |

## GPU Pipeline

### Per-Frame Data Flow

```
CPU: Update camera uniform (if moved) -> clear accumulation buffer
CPU: Update frame params (sample_count, frame_index)
GPU: Dispatch path_trace compute shader (8x8 workgroups)
GPU: Dispatch post_process compute shader (if effect active)
GPU: Blit output texture -> swapchain surface (fullscreen triangle)
GPU: egui render pass (UI overlay)
```

### Bind Groups

| Group | Binding | Type | Data |
|-------|---------|------|------|
| 0 | 0 | uniform | Camera + frame params |
| 0 | 1 | storage r/w | Accumulation buffer (vec4f per pixel) |
| 0 | 2 | storage texture (write) | Output (rgba8unorm) |
| 1 | 0 | storage read | Shape array |
| 1 | 1 | storage read | Material array |
| 1 | 2 | storage read | BVH nodes |
| 1 | 3 | storage read | BVH primitive indices |
| 1 | 4 | storage read | Light indices |
| 1 | 5 | storage read | Texture atlas pixels |
| 1 | 6 | storage read | Texture atlas infos |

### Post-Process Bind Group

| Group | Binding | Type | Data |
|-------|---------|------|------|
| 0 | 0 | uniform | Post-process params (width, height, effect list) |
| 0 | 1 | storage read | Accumulation buffer |
| 0 | 2 | storage texture (write) | Output texture |

### Shader Composition

WGSL has no `#include`. The `ShaderComposer` (`src/shaders/composer.rs`) resolves `// #import module_name` directives by concatenating shader files in dependency order with deduplication. Module names map to file paths: `figures::sphere` -> `figures/sphere.wgsl`.

## Rendering Algorithm

### Path Tracing Loop

For each pixel per frame:
1. Initialize PCG hash RNG seeded by (pixel_x, pixel_y, frame_index)
2. Generate camera ray with sub-pixel jitter (built-in AA)
3. For each bounce (up to 16, with Russian Roulette after 3):
   - Trace ray through BVH (closest-hit)
   - On miss: add sky contribution, break
   - On hit emissive: add MIS-weighted emission, break
   - Glass: Fresnel-weighted reflect/refract
   - NEE: sample random light, shadow ray, MIS-weighted direct contribution
   - BRDF importance sampling (GGX for specular, cosine for diffuse)
   - Russian Roulette termination (survival = max component of throughput)
4. Welford's progressive accumulation: `acc += (new - acc) / n`
5. ACES filmic tone mapping + sRGB gamma correction

### PBR Material Model (Cook-Torrance / GGX)

Unified material with: base_color, metallic, roughness, emission, IOR, transmission.

BRDF = diffuse + specular where:
- Diffuse: `(1 - metallic) * baseColor / PI`
- Specular: `D * G * F / (4 * NdotL * NdotV)`
- D = GGX NDF, G = Smith GGX, F = Fresnel-Schlick

### BVH Acceleration

- CPU: SAH-based binary BVH with 12-bin split search
- GPU: Stack-based traversal (32-entry stack), near-child-first ordering
- Shadow rays: same BVH with any-hit early termination
- SDF shapes: large bounding-box leaf nodes, sphere marching inside

## Supported Shape Types

| Type | Intersection Method |
|------|-------------------|
| Sphere | Half-b quadratic formula |
| Plane | Pre-normalized dot product |
| Cube | Ray-AABB slab test |
| Cylinder | Axis projection + slab caps |
| Cone | Pre-computed tan^2 |
| Torus | SDF sphere marching |
| Disc | Squared distance check |
| Triangle | Moller-Trumbore |
| Ellipsoid | Scaled-space sphere intersection |
| Paraboloid | SDF sphere marching |
| Hyperboloid | SDF sphere marching |
| Pyramid | SDF sphere marching |
| Tetrahedron | SDF sphere marching |
| Mebius | SDF sphere marching |
| Skybox | Procedural sky gradient |
| Mandelbulb | Trig-based IQ SDF + over-relaxation |
| Julia | Quaternion SDF + over-relaxation |
