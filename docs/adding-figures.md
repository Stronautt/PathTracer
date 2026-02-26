# Adding a New Figure Type

This guide walks through adding a new geometric primitive to the path tracer.

## Steps

### 1. Create the WGSL shader

Create `src/shaders/wgsl/figures/new_figure.wgsl`:

```wgsl
// #import types

fn intersect_new_figure(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    // Your intersection math here...
    // Use fig.position, fig.radius, fig.normal, etc.

    if /* hit condition */ {
        hit.hit = true;
        hit.t = t;
        hit.position = ray.origin + ray.direction * t;
        hit.normal = /* compute normal */;
        hit.uv = /* compute UV */;
    }

    return hit;
}
```

### 2. Add to dispatch shader

In `src/shaders/wgsl/figures/dispatch.wgsl`:

Add the import:
```wgsl
// #import figures::new_figure
```

Add the case in the switch:
```wgsl
case FIG_NEW_FIGURE: {
    hit = intersect_new_figure(ray, fig);
}
```

### 3. Add the figure type constant

In `src/shaders/wgsl/types.wgsl`:
```wgsl
const FIG_NEW_FIGURE: u32 = 11u;
```

### 4. Add to Rust FigureType enum

In `src/scene/figure.rs`:

```rust
pub enum FigureType {
    // ...existing types...
    NewFigure,
}
```

Update `as_u32()`, `label()`, and `ALL` to include the new variant.

### 5. Add AABB computation

In `src/accel/aabb.rs`, add a case to `figure_aabb()`:

```rust
FigureType::NewFigure => {
    let extent = Vec3::splat(fig.radius);
    Aabb::new(pos - extent, pos + extent)
}
```

### 6. Add UI button (optional)

The `add_figure_panel.rs` iterates `FigureType::ALL` automatically, so new types appear in the UI with no extra code.

## What you DON'T need to change

- Path tracer core (`path_trace.wgsl`)
- PBR materials (`materials.wgsl`)
- BVH traversal (`bvh.wgsl`)
- Rendering pipeline (`app.rs`, `frame.rs`)
- Tone mapping, camera, post-processing

The dispatch pattern isolates figure-specific code from the rendering pipeline.
