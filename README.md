# PathTracer

GPU-accelerated physically-based path tracer written in Rust using wgpu (WebGPU).

Renders scenes with geometric primitives, fractals (Mandelbulb, Julia set), and OBJ models using modern PBR rendering techniques.

## Features

- **PBR Materials**: Cook-Torrance/GGX microfacet BRDF with metallic-roughness workflow
- **Next Event Estimation**: Explicit direct light sampling for fast convergence
- **Multiple Importance Sampling**: Power heuristic MIS for low-variance rendering
- **BVH Acceleration**: SAH-based BVH with stack-based GPU traversal
- **Russian Roulette**: Unbiased path termination after minimum bounce depth
- **ACES Tone Mapping**: Filmic tone mapping with sRGB gamma correction and exposure control
- **11 Figure Types**: Sphere, Plane, Cube, Cylinder, Cone, Torus, Disc, Triangle, Skybox, Mandelbulb, Julia set
- **Glass/Transmission**: Fresnel-weighted reflection/refraction with configurable IOR
- **Progressive Rendering**: Welford's numerically stable accumulation
- **Real-time UI**: egui-based interface with material editing, post-processing effects, exposure control
- **Cross-platform**: Vulkan (Linux/Windows), Metal (macOS), DX12 (Windows)

## Requirements

- Rust 1.75+
- GPU with Vulkan 1.0, Metal, or DirectX 12 support

## Build & Run

```bash
# Build
cargo build --release

# Run with a scene file
cargo run --release -- resources/scenes/demo.json

# Run with logging
RUST_LOG=info cargo run --release -- resources/scenes/cornell_box.json
```

## Controls

| Input | Action |
|-------|--------|
| WASD | Move camera |
| Space / Ctrl | Move up / down |
| Right Mouse + drag | Look around |
| Shift | Sprint |
| Escape | Release mouse / quit |

## Scene Format

Scenes are JSON files. Example:

```json
{
  "camera": {
    "position": [0.0, 4.5, -16.0],
    "rotation": [-5.0, 0.0, 0.0],
    "fov": 60.0,
    "exposure": 1.0
  },
  "figures": [
    {
      "type": "sphere",
      "position": [0.0, 2.0, 0.0],
      "radius": 2.0,
      "material": {
        "base_color": [0.95, 0.93, 0.88],
        "metallic": 1.0,
        "roughness": 0.05
      }
    },
    {
      "type": "plane",
      "position": [0.0, 0.0, 0.0],
      "normal": [0.0, 1.0, 0.0],
      "material": {
        "base_color": [0.8, 0.8, 0.8],
        "roughness": 0.9
      }
    }
  ]
}
```

### Material Properties

| Property | Type | Range | Default | Description |
|----------|------|-------|---------|-------------|
| `base_color` | [f32; 3] | [0,1] | [0.8, 0.8, 0.8] | Albedo color |
| `metallic` | f32 | [0,1] | 0.0 | 0=dielectric, 1=metal |
| `roughness` | f32 | [0,1] | 0.5 | 0=mirror, 1=matte |
| `emission` | [f32; 3] | [0,inf) | [0,0,0] | Emissive color |
| `emission_strength` | f32 | [0,inf) | 0.0 | Emission multiplier |
| `ior` | f32 | [1,3] | 1.5 | Index of refraction |
| `transmission` | f32 | [0,1] | 0.0 | 0=opaque, 1=glass |

## Sample Scenes

- `demo.json` - Showcase scene with metal, glass, and diffuse spheres
- `cornell_box.json` - Classic Cornell box with glass and metal spheres
- `fractals.json` - Mandelbulb and Julia set fractals

## Architecture

See [docs/architecture.md](docs/architecture.md) for the full module map, GPU pipeline, and rendering algorithm details.

See [docs/adding-figures.md](docs/adding-figures.md) for a guide on adding new figure types.

## Tech Stack

| Component | Library |
|-----------|---------|
| GPU compute | wgpu (Vulkan/Metal/DX12) |
| Shading language | WGSL |
| Windowing | winit |
| GUI | egui |
| Scene parsing | serde + serde_json |
| OBJ loading | tobj |
| Math | glam |
| Image I/O | image |
