# Architecture

## Overview

PathTracer is a GPU-accelerated physically-based path tracer built with Rust and wgpu (WebGPU). It runs compute shaders on the GPU via Vulkan/Metal/DX12 backends.

## Module Map

```
src/
  main.rs         Entry point: CLI args, winit event loop
  app.rs          Application state, init, per-frame update/render, resize

  gpu/
    context.rs    wgpu instance/adapter/device/queue/surface
    pipeline.rs   Compute + render pipeline creation helpers
    buffers.rs    GPU buffer creation and update utilities

  render/
    frame.rs      Per-frame dispatch: trace -> post-process -> blit
    accumulator.rs  Progressive refinement sample counting and reset
    post_process.rs  Post-processing effect enum

  shaders/
    composer.rs   WGSL "// #import module" preprocessor (Bevy-style)
    wgsl/         All WGSL shader source files (see Shader Pipeline below)

  scene/
    scene.rs      Scene struct (camera config, figures, model refs)
    loader.rs     JSON deserialization via serde
    exporter.rs   JSON serialization (scene save)
    figure.rs     FigureType enum, Figure struct, GpuFigure
    material.rs   PBR Material struct, GpuMaterial

  accel/
    aabb.rs       Axis-aligned bounding box, per-figure AABB computation
    bvh.rs        SAH-based BVH construction, flat GPU node array

  model/
    obj_loader.rs OBJ file loading via tobj -> triangle figures

  camera/
    camera.rs     Camera state, quaternion orientation, GPU struct
    controller.rs FPS-style WASD + mouse controller

  ui/
    mod.rs        egui integration, draw_ui entry point
    toolbar.rs    Pause, screenshot, save, FPS, exposure
    object_editor.rs  PBR material sliders, position editor
    effects_panel.rs  Post-processing effect toggles
    add_figure_panel.rs  Add new figure buttons

  input/
    handler.rs    Keyboard/mouse event -> state flags

  io/
    screenshot.rs   Read GPU buffer -> save PNG
    texture_atlas.rs  Pack textures into flat GPU buffer
```

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
| 1 | 0 | storage read | Figure array |
| 1 | 1 | storage read | Material array |
| 1 | 2 | storage read | BVH nodes |
| 1 | 3 | storage read | BVH primitive indices |
| 1 | 4 | storage read | Light indices |

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
- SDF figures: large bounding-box leaf nodes, sphere marching inside

## Supported Figure Types

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
| Skybox | Procedural sky gradient |
| Mandelbulb | Trig-free triplex algebra + over-relaxation |
| Julia | Quaternion SDF + over-relaxation |
