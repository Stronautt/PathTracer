# Adding a New Shape Type

This guide walks through adding a new geometric primitive to the path tracer.

## Steps

### 1. Create the WGSL shader

Create `src/shaders/wgsl/figures/new_shape.wgsl`:

```wgsl
// #import types

fn intersect_new_shape(ray: Ray, fig: Figure) -> HitRecord {
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
// #import figures::new_shape
```

Add the case in the switch:
```wgsl
case FIG_NEW_SHAPE: {
    hit = intersect_new_shape(ray, fig);
}
```

### 3. Add the shape type constant

In `src/shaders/wgsl/types.wgsl`:
```wgsl
const FIG_NEW_SHAPE: u32 = 17u;
```

### 4. Add to Rust ShapeType enum

In `src/scene/shape.rs`:

```rust
pub enum ShapeType {
    // ...existing types...
    NewShape = 17,
}
```

Update `label()`, `ALL`, and the `ELEMENTARY`/`COMPLEX` category constants to include the new variant.

### 5. Add AABB computation

In `src/accel/aabb.rs`, add a case to `shape_aabb()`:

```rust
ShapeType::NewShape => {
    let extent = Vec3::splat(shape.radius);
    Aabb::new(pos - extent, pos + extent)
}
```

### 6. UI

The toolbar's "Add Shape" menu reads from `ShapeType::ELEMENTARY` and `ShapeType::COMPLEX`, so add your new type to the appropriate category and it will appear automatically.

### 7. (Optional) Set default spawn parameters

In `src/app/scene_ops.rs`, add a case to `add_shape()` to customize any default parameters for the new shape (radius, height, etc.):

```rust
ShapeType::NewShape => {
    shape.radius = 1.0;
    // ...
}
```

## What you DON'T need to change

- Path tracer core (`path_trace.wgsl`)
- PBR materials (`materials.wgsl`)
- BVH traversal (`bvh.wgsl`)
- App module (`src/app.rs`, `src/app/rendering.rs`, `src/app/state.rs`)
- Render pipeline (`frame.rs`)
- Tone mapping, camera, post-processing
- Constants (`constants.rs`)

The dispatch pattern isolates shape-specific code from the rendering pipeline.
