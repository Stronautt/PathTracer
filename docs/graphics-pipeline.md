# PathTracer Graphics Pipeline

Comprehensive documentation of the GPU-accelerated path tracing pipeline used in PathTracer. This document covers every stage from scene data upload through final display, including the mathematical foundations of the rendering algorithms.

---

## Table of Contents

1. [Pipeline Overview](#1-pipeline-overview)
2. [Path Tracing Algorithm](#2-path-tracing-algorithm)
3. [BVH Acceleration Structure](#3-bvh-acceleration-structure)
4. [Materials & BRDF](#4-materials--brdf)
5. [Direct Lighting (NEE + MIS)](#5-direct-lighting-nee--mis)
6. [Shape Intersection Algorithms](#6-shape-intersection-algorithms)
7. [Progressive Accumulation](#7-progressive-accumulation)
8. [Tone Mapping & Post-Processing](#8-tone-mapping--post-processing)
9. [GPU Data Layout](#9-gpu-data-layout)
10. [Shader Composition](#10-shader-composition)

---

## 1. Pipeline Overview

### High-Level Frame Pipeline

```
                        CPU (Rust / wgpu)                          GPU (WGSL Compute + Render)
  ========================================================================================

  Scene YAML -----> Shape/Material/BVH -----> GPU Buffers
                    construction (CPU)          upload
                         |
                         v
  Camera update -----> Uniform buffer ----+
                       write              |
                                          |
                    +---------------------+
                    |
                    v
              [Compute Pass 1: Path Trace]
              |  - Generate camera ray per pixel (with sub-pixel jitter)
              |  - BVH traversal to find closest intersection
              |  - Bounce loop: BRDF sampling, NEE, Russian Roulette
              |  - Write radiance into accumulation buffer
              |  - Tone map accumulated result -> output texture
              |
              v
              [Compute Pass 2: Post-Processing]  (optional)
              |  - Read from accumulation buffer
              |  - Apply chained effects (FXAA, Sepia, Comic, etc.)
              |  - Overwrite output texture
              |
              v
              [Render Pass 1: Blit]
              |  - Fullscreen triangle samples output texture
              |  - Writes to swapchain surface
              |
              v
              [Render Pass 2: egui UI Overlay]
              |  - Toolbar, object editor, settings panels
              |  - Composited on top of the rendered image
              |
              v
            Present to display
```

### Per-Frame Execution Flow (Rust Side)

The frame loop lives in `src/app/rendering.rs` (`update_and_render`). Each frame:

1. **Input & Camera** -- Process keyboard/mouse input, update camera position and orientation. If the camera moved, reset accumulation.
2. **UI** -- Run the egui immediate-mode UI. Collect any actions (add shape, delete shape, change exposure, etc.).
3. **Apply UI Actions** -- Rebuild GPU buffers if the scene changed (shapes, materials, BVH, textures).
4. **Advance Accumulator** -- Increment sample count. If accumulation was reset, clear the accumulation buffer on the GPU with `encoder.clear_buffer()`.
5. **Update Camera Uniform** -- Write the new `GpuCamera` struct to the uniform buffer.
6. **Dispatch Path Trace** -- 8x8 workgroups covering every pixel.
7. **Dispatch Post-Processing** -- Only if the user has enabled effects.
8. **Blit Pass** -- Render pass that draws a fullscreen triangle sampling the output texture onto the swapchain surface.
9. **egui Pass** -- Render pass that composites the UI overlay (loads the existing surface contents, draws on top).
10. **Submit & Present** -- Submit the command encoder, present the swapchain texture.

### Data Flow Summary

```
Scene YAML/JSON
      |
      v
  Rust loader (serde)
      |
      +---> Vec<Shape>  ---------> Vec<GpuShape>   --------> shape_buffer        (storage, read-only)
      +---> Vec<Material> -------> Vec<GpuMaterial> --------> material_buffer     (storage, read-only)
      +---> Vec<Aabb>  ----------> Bvh::build()    --------> bvh_node_buffer     (storage, read-only)
      |                                             --------> bvh_prim_buffer     (storage, read-only)
      +---> light indices  -----------------------------> light_index_buffer  (storage, read-only)
      +---> texture paths  ------> TextureAtlas    --------> tex_pixels_buffer   (storage, read-only)
                                                   --------> tex_infos_buffer    (storage, read-only)

Camera state  ---> GpuCamera  ---> camera_buffer (uniform)

Per-pixel output:
  accumulation_buffer (storage, read-write)  -- vec4f per pixel, running mean
  output_texture (rgba8unorm, write-only)    -- final tonemapped/post-processed image
```

---

## 2. Path Tracing Algorithm

Path tracing solves the rendering equation through Monte Carlo integration. Instead of computing all possible light paths analytically (which is intractable), we trace random paths from the camera into the scene, bouncing off surfaces, and average many such paths to converge on the correct image.

### The Rendering Equation

At a high level, the color of each pixel is determined by:

> **Outgoing light = Emitted light + Integral of (incoming light * surface reflectance * geometric term) over the hemisphere**

We estimate this integral by tracing random rays. Each additional sample reduces noise. The more samples, the cleaner the image.

### Ray Generation

**File:** `src/shaders/wgsl/camera.wgsl`

Each pixel gets a camera ray. The camera is defined by pre-computed basis vectors (`right`, `up`, `forward`) and a `focal_length` that controls the field of view.

```
generate_ray(camera, pixel):
    1. Add random sub-pixel jitter in [-0.5, +0.5] for anti-aliasing
    2. Convert pixel to Normalized Device Coordinates (NDC):
         ndc_x = (2 * px / width - 1) * aspect_ratio
         ndc_y = 1 - 2 * py / height
    3. Compute ray direction:
         dir = normalize(right * ndc_x + up * ndc_y + forward * focal_length)
    4. Return Ray(camera.position, dir)
```

The sub-pixel jitter means each frame's rays hit slightly different points within each pixel. Over many frames, this naturally produces anti-aliasing through the progressive accumulation (Section 7).

This is a pinhole camera model (no depth of field). All rays originate from a single point. The `focal_length` parameter acts as the cotangent of the half-FOV angle, controlling how wide or narrow the view is.

### The Bounce Loop

**File:** `src/shaders/wgsl/path_trace.wgsl` -- `trace_path()`

The core of the path tracer. We follow each ray as it bounces around the scene, accumulating light contributions:

```
trace_path(ray):
    throughput = (1, 1, 1)     // How much light the path can still carry
    radiance = (0, 0, 0)      // Accumulated color
    specular_bounce = true     // Whether the last bounce was specular

    for bounce in 0..MAX_BOUNCES(16):
        hit = trace_bvh(ray)   // Find closest intersection via BVH

        if no hit:
            radiance += throughput * sample_skybox(ray.direction)
            break

        // Apply texture to material
        material.base_color *= sample_texture(material.texture_id, hit.uv)

        // Emissive surface -- we've hit a light
        if material.emission_strength > 0:
            radiance += throughput * emission
            break

        // Glass/transparent material -- handle separately
        if material.transmission > 0.5:
            sample = sample_glass(wo, n, material)
            throughput *= sample.brdf_cos
            ray = new Ray from hit point along sample.direction
            continue

        // NEE: Explicitly sample a light source (Section 5)
        if material.roughness > 0.04 and lights exist:
            radiance += throughput * direct_lighting_contribution

        // BRDF sampling: pick a random bounce direction
        sample = sample_brdf(wo, n, material)
        throughput *= sample.brdf_cos / sample.pdf
        ray = new Ray from hit point along sample.direction

        // Russian Roulette (after 3 bounces)
        if bounce >= 3:
            survival = min(max_component(throughput), 0.95)
            if random() > survival:
                break                     // Terminate path
            throughput /= survival        // Compensate surviving paths

        // Firefly clamping
        if luminance(throughput) > 100:
            throughput *= 100 / luminance(throughput)

    return max(radiance, 0)
```

Key concepts:

- **Throughput** tracks how much of the original ray's energy remains after each bounce. It gets multiplied by the BRDF reflectance at each surface.
- **Radiance** is the final color we are building up. Light is added when we hit an emissive surface or the sky.
- **Russian Roulette** is a probabilistically unbiased way to terminate paths. After 3 bounces, paths with low throughput have a chance of being killed. Surviving paths get their throughput boosted to compensate, keeping the result unbiased.
- **Firefly clamping** caps extreme throughput values (from unlikely sampling events) to prevent bright firefly artifacts. This introduces a small amount of bias but greatly improves visual quality during convergence.

### Random Number Generation

**File:** `src/shaders/wgsl/random.wgsl`

The renderer uses a PCG (Permuted Congruential Generator) hash for random numbers. Each pixel's RNG is seeded from a combination of:
- Pixel coordinates (x, y)
- Frame index

This ensures every pixel gets a unique, well-distributed random sequence each frame, avoiding visible correlation patterns between neighboring pixels.

```
seed = pcg_hash(pixel.x + pixel.y * 65536 + frame * 16777259)
```

The `rand_f32()` function produces uniform floats in [0, 1) by hashing the current state and dividing by 2^32.

---

## 3. BVH Acceleration Structure

Without acceleration, testing every ray against every shape is O(N) per ray. A Bounding Volume Hierarchy (BVH) reduces this to O(log N) on average by organizing shapes into a binary tree of bounding boxes.

### What is a BVH?

Think of it as a spatial index. The root node's bounding box contains the entire scene. Each internal node splits its shapes into two child groups, each with their own tighter bounding box. Leaf nodes contain a small number of actual shapes (up to 4 in this implementation).

```
                [Root: entire scene AABB]
                /                        \
    [Left: half the shapes]      [Right: other half]
       /          \                  /          \
   [Leaf: 2]   [Leaf: 3]      [Leaf: 1]    [Inner]
   shapes       shapes         shape        /     \
                                        [Leaf:2] [Leaf:2]
```

When tracing a ray, we first test against the root's bounding box. If the ray misses, we skip the entire tree. If it hits, we descend into whichever child the ray hits first, and so on. Most rays skip the vast majority of shapes this way.

### Surface Area Heuristic (SAH) Construction

**File:** `src/accel/bvh.rs`

The quality of a BVH depends heavily on how we choose to split shapes at each node. The Surface Area Heuristic estimates the cost of a split based on a key insight: the probability that a random ray hits a bounding box is proportional to its surface area.

The SAH cost for splitting N primitives into groups L and R is:

```
cost = count_L * area_L + count_R * area_R
```

We want to find the split that minimizes this cost.

### Binned SAH Algorithm

Testing every possible split point would be O(N^2). Instead, this implementation uses **binned SAH** with 12 bins per axis, reducing the cost to O(N) per axis:

```
For each axis (X, Y, Z):
    Phase 1: Bin primitives by centroid position
        - Divide the axis extent into 12 equal bins
        - Assign each primitive to a bin based on its centroid
        - Track the bounding box and count for each bin

    Phase 2: Right-to-left sweep
        - Accumulate bounding boxes and counts from right to left
        - Store the right-side area and count for each possible split

    Phase 3: Left-to-right sweep
        - Accumulate from left to right
        - Evaluate SAH cost at each of the 11 possible split positions
        - Track the minimum cost split

Choose the axis and split position with the lowest cost.
```

**Constants:** `BVH_NUM_BINS = 12`, `BVH_LEAF_MAX_PRIMS = 4`

When a partition is degenerate (all primitives end up on one side), the builder falls back to a median split (half on each side).

### Flat GPU Layout

The BVH tree is flattened into a contiguous array for GPU consumption. The layout uses an implicit left-child convention:

- For any **inner node** at index `i`, its left child is always at `i + 1`
- The right child index is stored explicitly in `left_or_prim`
- **Leaf nodes** have `prim_count > 0`, and `left_or_prim` stores the first primitive index into the `bvh_prims` indirection array

```
struct BvhNode {
    aabb_min: vec3f,
    left_or_prim: u32,    // Inner: right child index  |  Leaf: first prim offset
    aabb_max: vec3f,
    prim_count: u32,      // 0 = inner node  |  >0 = leaf with this many prims
}
```

### Stack-Based GPU Traversal

**File:** `src/shaders/wgsl/bvh.wgsl` -- `trace_bvh_positive()`

The GPU traversal uses an explicit stack (array of 32 u32 indices) since GPU shaders cannot use recursion:

```
trace_bvh_positive(ray):
    closest.t = MAX_T
    stack = [0]    // Start with root node

    while stack is not empty:
        node = pop from stack

        if ray misses node's AABB, or AABB is farther than closest hit:
            skip

        if node is a leaf:
            for each primitive in this leaf:
                test ray-shape intersection
                update closest if nearer

        else (inner node):
            test both children's AABBs
            push far child first, then near child
            (so near child is processed next -- depth-first, near-first)

    return closest hit
```

The **near-child-first** ordering is a crucial optimization. By processing the nearer child first, we are more likely to find a close hit early, which allows us to skip the far child entirely (since its AABB distance exceeds our current closest hit).

### Shadow Ray Traversal

Shadow rays use a separate `trace_shadow()` function with **any-hit early termination**. Unlike closest-hit traversal, we only need to know if *anything* blocks the path to the light, so we return `true` as soon as we find any intersection (skipping negative/CSG shapes and skybox).

### CSG Subtraction

The BVH traversal supports basic Constructive Solid Geometry (CSG) subtraction. Shapes marked with `csg_op = 1` (subtraction) are "negative" -- they carve holes in other shapes:

1. Find the closest positive-shape hit via normal BVH traversal
2. Check if the hit point is inside any negative shape
3. If so, find the exit point of the negative shape and advance the ray past it
4. Repeat (up to 8 attempts) until we find a hit outside all negative shapes

This allows effects like cutting a spherical hole through a cube.

---

## 4. Materials & BRDF

### PBR Material Model

**File:** `src/shaders/wgsl/materials.wgsl`

Each surface in the scene has a physically-based material with these parameters:

| Parameter | Range | Description |
|-----------|-------|-------------|
| `base_color` | RGB [0,1] | Surface color (albedo for dielectrics, reflectance color for metals) |
| `metallic` | [0, 1] | 0 = dielectric (plastic, wood), 1 = metal (gold, copper) |
| `roughness` | [0, 1] | 0 = mirror-smooth, 1 = completely rough |
| `transmission` | [0, 1] | 0 = opaque, >0.5 = glass/transparent |
| `ior` | ~1.0-2.5 | Index of refraction (glass ~1.5, water ~1.33, diamond ~2.42) |
| `emission` | RGB | Emission color |
| `emission_strength` | >=0 | Emission intensity multiplier |
| `texture_id` | int | Index into texture atlas (-1 = no texture) |

### Cook-Torrance Microfacet BRDF

The BRDF (Bidirectional Reflectance Distribution Function) describes how light reflects off a surface. This renderer uses the Cook-Torrance microfacet model, which models a surface as composed of tiny mirror-like facets oriented in various directions.

The full BRDF is a sum of diffuse and specular components:

```
BRDF = k_d * Lambertian_diffuse + Cook-Torrance_specular
```

Where:
- `k_d = (1 - Fresnel) * (1 - metallic)` -- metals have no diffuse component
- `Lambertian_diffuse = base_color / pi`
- `Cook-Torrance_specular = (D * G * F) / (4 * dot(N,L) * dot(N,V))`

The three terms D, G, F in the specular component are:

#### GGX Normal Distribution Function (D)

The NDF describes how the microfacet normals are distributed. GGX (also called Trowbridge-Reitz) has a longer tail than Beckmann, producing more realistic specular highlights with a soft falloff:

```
D(h) = alpha^2 / (pi * ((n.h)^2 * (alpha^2 - 1) + 1)^2)
```

Where `alpha = roughness^2` and `h` is the half-vector between the view and light directions.

- When roughness is near 0, the distribution is a sharp spike (mirror reflection)
- When roughness is near 1, the distribution is broad (diffuse-like highlights)

#### Smith Geometry Function (G)

The geometry function accounts for self-shadowing of microfacets -- rough surfaces have microscopic hills and valleys that block light. The Smith form separates this into two independent terms (masking by the view direction, and shadowing by the light direction):

```
G(l, v) = G1(l) * G1(v)

G1(v) = 2 * (n.v) / ((n.v) + sqrt(alpha^2 + (1 - alpha^2) * (n.v)^2))
```

#### Fresnel Effect (F) -- Schlick Approximation

The Fresnel effect describes how surfaces become more reflective at grazing angles. Look at a lake: straight down you see through the water, but at a shallow angle it acts like a mirror.

```
F(cos_theta) = F0 + (1 - F0) * (1 - cos_theta)^5
```

Where `F0` is the reflectance at normal incidence:
- Dielectrics: `F0 = 0.04` (about 4% reflection straight-on)
- Metals: `F0 = base_color` (metals tint their reflections)

The `mix(0.04, base_color, metallic)` interpolation handles the transition.

### Importance Sampling

Randomly sampling directions uniformly over the hemisphere would be extremely wasteful -- most of the BRDF's energy is concentrated in a small region. Importance sampling generates random directions that are distributed proportionally to the BRDF, dramatically reducing noise.

The renderer uses a two-lobe sampling strategy:

1. **Specular lobe** (probability `spec_prob`): Sample a half-vector from the GGX distribution, then reflect the view direction around it. This naturally concentrates samples where the specular BRDF is large.

2. **Diffuse lobe** (probability `1 - spec_prob`): Cosine-weighted hemisphere sampling, which distributes samples proportionally to `cos(theta)` -- matching the Lambertian BRDF shape.

The probability of choosing specular vs. diffuse is based on metallic:
```
spec_weight = mix(0.04, 1.0, metallic)
spec_prob = max(spec_weight, 0.25)    // Always give specular at least 25% chance
```

The final PDF combines both lobes:
```
pdf = spec_prob * ggx_pdf + (1 - spec_prob) * cosine_pdf
```

This combined PDF ensures the Monte Carlo estimator `BRDF * cos(theta) / pdf` has low variance regardless of which lobe generated the sample.

### Glass / Dielectric Materials

**Handling in:** `sample_glass()`

When `transmission > 0.5`, the material is treated as glass. The Fresnel equation determines whether the ray reflects or refracts:

1. Compute the Fresnel reflectance at the current angle
2. With probability = Fresnel, **reflect** the ray
3. Otherwise, **refract** using Snell's law:
   ```
   sin(theta_t) = (eta_i / eta_t) * sin(theta_i)
   ```
   Where `eta_i` and `eta_t` are the refractive indices of the two media.
4. If `sin(theta_t) > 1` (total internal reflection), reflect instead

The implementation tracks whether the ray is entering or exiting the glass to correctly swap the IOR ratio.

Glass bounces are always marked as specular (no NEE -- you cannot meaningfully sample a light through refraction).

---

## 5. Direct Lighting (NEE + MIS)

### The Problem with Pure Path Tracing

In naive path tracing, a ray must randomly bounce into a light source to contribute color. For small lights, this is extremely unlikely, causing noisy images that take thousands of samples to converge. A room lit by a small lamp would be almost entirely black for the first hundred samples.

### Next Event Estimation (NEE)

NEE, also called "direct light sampling" or "explicit light sampling," solves this by directly connecting each hit point to a light source:

```
At each non-specular bounce:
    1. Pick a random light from the light list
    2. Sample a random point on the light's surface
    3. Cast a shadow ray from the hit point to the light point
    4. If not occluded:
        contribution = Le * BRDF * cos(theta) / (light_pdf * num_lights)
        radiance += throughput * contribution
```

The implementation samples sphere lights uniformly over their surface area:

```
sample_sphere_light(light, hit_pos):
    Pick random point uniformly on the sphere surface
    Return light.position + random_direction * light.radius
```

The area PDF is simply `1 / (4 * pi * radius^2)`, which is then converted to a solid-angle PDF for proper integration:

```
solid_angle_pdf = area_pdf * distance^2 / cos(angle_at_light)
```

NEE is skipped for nearly-specular surfaces (`roughness <= 0.04`) because the BRDF is so narrow that the chance of the light sample falling within it is vanishingly small.

### Multiple Importance Sampling (MIS)

**File:** `src/shaders/wgsl/mis.wgsl`

NEE and BRDF sampling are complementary strategies:
- **NEE** works great for small, bright lights but poorly for large area lights
- **BRDF sampling** works great for mirror-like surfaces but poorly for small lights

MIS combines both strategies using the **power heuristic**, which weights each sample based on how likely it was under both strategies:

```
mis_weight(pdf_a, pdf_b):
    return pdf_a^2 / (pdf_a^2 + pdf_b^2)
```

The NEE contribution is weighted by:
```
w = mis_weight(light_solid_pdf * num_lights, brdf_pdf)
```

This automatically reduces the weight of the NEE sample when the BRDF sampling would have been equally likely to find the light (avoiding double-counting), and gives full weight when the light is in a direction the BRDF sampler would be unlikely to explore.

When a BRDF-sampled ray directly hits a light (emission check in the bounce loop), it currently receives full weight on the first bounce or specular bounces. For a fully rigorous MIS implementation, these hits would also be weighted, but the current approach is a practical approximation that works well.

---

## 6. Shape Intersection Algorithms

The renderer supports 17 shape types, using three categories of intersection algorithms.

### Category 1: Analytical Ray Intersections

These shapes have closed-form ray intersection solutions -- we solve a mathematical equation directly.

#### Sphere (`figures/sphere.wgsl`)

The fastest intersection. Uses the optimized "half-b" quadratic formulation:

```
oc = ray.origin - sphere.center
half_b = dot(oc, ray.direction)
c = dot(oc, oc) - radius^2
discriminant = half_b^2 - c

If discriminant < 0: no intersection
t = -half_b - sqrt(discriminant)    // Near hit
If t < 0: t = -half_b + sqrt(discriminant)    // Far hit (we're inside)
```

UV mapping uses spherical coordinates: `u = atan2(z, x)`, `v = asin(y)`.

#### Plane (`figures/plane.wgsl`)

Simple ray-plane intersection:

```
t = dot(plane.position - ray.origin, plane.normal) / dot(plane.normal, ray.direction)
```

The normal is flipped to always face the ray. UV coordinates are generated by projecting the hit point onto an orthonormal basis built from the plane normal.

#### Disc (`figures/disc.wgsl`)

A plane intersection followed by a distance check (using squared distance to avoid a `sqrt`):

```
t = plane intersection with disc's normal
if distance_squared(hit_point, disc.center) > radius^2: miss
```

#### Cube (`figures/cube.wgsl`)

Uses the classic **slab test** -- the cube is the intersection of three pairs of parallel planes (slabs). The ray's entry and exit times through each slab are computed, and the cube is hit if the overall entry time is before the overall exit time:

```
inv_dir = 1 / ray.direction
t1 = (box_min - ray.origin) * inv_dir
t2 = (box_max - ray.origin) * inv_dir

t_near = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z))
t_far  = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z))

Hit if t_near <= t_far and t_far > 0
```

The normal is determined by which face was hit (the axis with the largest component in the local hit position).

#### Cylinder (`figures/cylinder.wgsl`)

Intersects a finite cylinder with caps. The body intersection reduces to a 2D circle intersection by projecting out the axis component:

```
d_perp = ray.direction - dot(ray.direction, axis) * axis    // Perpendicular to axis
oc_perp = oc - dot(oc, axis) * axis

Solve quadratic: |d_perp * t + oc_perp|^2 = radius^2
Check if hit point's projection onto axis is within [-height/2, +height/2]
```

Then separately test the two disc caps.

#### Cone (`figures/cone.wgsl`)

Similar to cylinder but with a varying radius. The cone equation in terms of the apex and axis is:

```
dot(p - apex, axis)^2 = cos^2(half_angle) * dot(p - apex, p - apex)
```

Where `cos^2 = 1 / (1 + tan^2)` and `tan^2` is pre-computed from the cone's base radius and height. Includes a base cap intersection.

#### Triangle (`figures/triangle.wgsl`) -- Moller-Trumbore Algorithm

The gold standard for ray-triangle intersection, computing barycentric coordinates without building the triangle's plane first:

```
e1 = v1 - v0
e2 = v2 - v0
h = cross(ray.direction, e2)
a = dot(e1, h)                     // If near zero, ray is parallel
f = 1 / a
s = ray.origin - v0
u = f * dot(s, h)                  // First barycentric coordinate
q = cross(s, e1)
v = f * dot(ray.direction, q)      // Second barycentric coordinate
t = f * dot(e2, q)                 // Distance

Hit if u in [0,1], v in [0,1], u+v <= 1, t > 0
```

Per-vertex UV coordinates are packed as half-floats in the Figure struct's padding fields and interpolated using barycentric coordinates.

#### Ellipsoid (`figures/ellipsoid.wgsl`)

The ellipsoid intersection transforms the ray into a unit-sphere space by dividing by the three radii `(rx, ry, rz)`, solves the standard sphere equation, and then transforms back:

```
Transformed ray: origin' = (origin - center) / radii, direction' = direction / radii
Solve unit sphere intersection in transformed space
Normal: gradient of (x/rx)^2 + (y/ry)^2 + (z/rz)^2 = 1
```

#### Paraboloid (`figures/paraboloid.wgsl`)

Intersects the quadric surface `x^2 + z^2 = r * y`, capped at `y = height`, with a top cap disc.

#### Hyperboloid (`figures/hyperboloid.wgsl`)

Intersects the one-sheet hyperboloid `x^2/r^2 + z^2/r^2 - y^2/r^2 = 1`, capped at `y = +/- height/2`, with top and bottom cap discs.

#### Pyramid and Tetrahedron (`figures/pyramid.wgsl`, `figures/tetrahedron.wgsl`)

Both are implemented as collections of triangles using Moller-Trumbore:
- **Pyramid**: 4 triangular side faces + 2 triangles for the square base (6 tests total)
- **Tetrahedron**: 4 triangular faces with vertices computed from the circumradius

### Category 2: SDF Ray Marching (Sphere Tracing)

Shapes without closed-form intersections use Signed Distance Functions (SDFs). An SDF returns the distance from any point to the nearest surface: positive outside, negative inside, zero on the surface.

**Sphere tracing** marches along the ray in steps, where each step size equals the SDF value at the current position. Since the SDF guarantees nothing is closer than the returned distance, this is safe and converges quickly:

```
Sphere tracing:
    t = starting_t
    for up to N iterations:
        p = ray.origin + ray.direction * t
        d = sdf(p)
        if |d| < epsilon * t:    // Distance-relative convergence
            HIT at t
        t += d
        if t > max_t:
            MISS
```

The convergence test `|d| < epsilon * t` uses a distance-relative threshold, which is more robust than a fixed epsilon for objects at varying distances.

**Normal estimation** uses the **tetrahedron trick** -- evaluating the SDF at four carefully chosen points around the hit to estimate the gradient (which equals the surface normal). This requires only 4 SDF evaluations instead of the 6 needed by central differences:

```
e = (1, -1) * 0.5773 * epsilon
normal = normalize(
    e.xyy * sdf(p + e.xyy) +
    e.yyx * sdf(p + e.yyx) +
    e.yxy * sdf(p + e.yxy) +
    e.xxx * sdf(p + e.xxx)
)
```

#### Torus (`figures/torus.wgsl`)

The torus SDF is elegant and compact:

```
sdf_torus(p, major_r, minor_r):
    q = vec2(length(p.xz) - major_r, p.y)
    return length(q) - minor_r
```

Uses **over-relaxation sphere tracing** (omega = 1.4): steps are multiplied by a factor > 1 to converge faster. If the over-step causes the ray to pass through the surface (SDF goes negative), it backtracks and takes a smaller step:

```
if d < 0 and prev_d > 0:    // Overstepped!
    t -= prev_d * (omega - 1)    // Backtrack the over-step part
    d2 = sdf at new position
    t += d2                       // Normal step from here
else:
    t += d * omega               // Over-relaxed step
```

#### Mobius Strip (`figures/mebius.wgsl`)

The Mobius strip SDF uses a parametric construction:
1. Find the closest angle on the center circle
2. Compute the local frame (radial and up directions)
3. Apply the half-twist: the strip's "up" direction rotates by half the angle around the circle
4. Measure the distance to the rectangular cross-section in the local (u, v) frame

Standard sphere tracing (no over-relaxation) with up to 256 iterations.

### Category 3: Fractal SDFs

#### Mandelbulb (`figures/mandelbulb.wgsl`)

The Mandelbulb is a 3D analog of the Mandelbrot set, using "triplex algebra" (spherical coordinates) to raise a 3D point to a power:

```
sdf_mandelbulb(p, power, max_iter):
    w = p
    m = dot(w, w)
    dz = 1

    for max_iter iterations:
        // Running derivative for distance estimation
        dz = power * |w|^(power-1) * dz + 1

        // Raise to power using spherical coordinates
        r = |w|
        theta = power * acos(w.y / r)
        phi = power * atan2(w.x, w.z)
        w = p + r^power * (sin(theta)*sin(phi), cos(theta), sin(theta)*cos(phi))

        if |w|^2 > 256: break    // Escaped

    // Hubbard-Douady distance estimate
    return 0.25 * log(|w|^2) * |w| / dz
```

This is Inigo Quilez's formulation. The distance estimate uses the running derivative `dz` to convert the escape-time iteration count into an approximate distance to the fractal surface.

Uses over-relaxation sphere tracing (omega = 1.3) with up to 256 steps.

#### Quaternion Julia Set (`figures/julia.wgsl`)

A 4D Julia set sliced into 3D. Points are embedded as quaternions `(x, y, z, 0)` and iterated with `z = z^2 + c`:

```
sdf_julia(p, c, max_iter):
    z = quaternion(p, 0)
    dz = quaternion(1, 0, 0, 0)

    for max_iter iterations:
        dz = 2 * z * dz          // Quaternion multiplication
        z = z * z + c             // Quaternion multiplication + add
        if |z|^2 > 16: break

    return 0.5 * |z| * log(|z|) / |dz|
```

The Julia constant `c` is a 4D quaternion stored in `rotation.xyz` and `radius2`.

Also uses over-relaxation sphere tracing (omega = 1.3) with up to 256 steps.

### Skybox

**File:** `figures/skybox.wgsl`

The skybox is not a traditional intersection -- it is sampled on ray miss via `sample_skybox()`. It searches the shape list for a skybox figure with a texture. If found, the ray direction is converted to equirectangular UV coordinates and the texture is sampled. If no skybox texture exists, a procedural sky gradient is returned:

```
t = 0.5 * (direction.y + 1)
sky = mix(white, light_blue, t) * 0.3
```

---

## 7. Progressive Accumulation

### Welford's Online Mean Algorithm

**File:** `src/shaders/wgsl/path_trace.wgsl` (lines 44-49), `src/render/accumulator.rs`

Each frame traces one sample per pixel. Rather than storing and averaging all samples (which would require unbounded memory), the renderer uses Welford's online algorithm to compute a running mean:

```
accumulated = previous + (new_sample - previous) / n
```

Where `n` is the current sample count.

This is numerically stable -- unlike the naive `sum / n` approach, it does not suffer from floating-point precision loss when adding a small new sample to a large accumulated sum. This matters because the accumulation buffer may hold thousands of frames of integrated values.

The accumulation buffer stores one `vec4f` per pixel (16 bytes per pixel). When the camera moves or the scene changes, the CPU-side `Accumulator` marks the buffer dirty, and the next frame clears it with `encoder.clear_buffer()` (a GPU-side clear to avoid transferring a large zeroed array from CPU).

### Convergence Behavior

The image noise decreases proportionally to `1 / sqrt(n)` where `n` is the sample count. This means:
- 1 sample: very noisy
- 100 samples: 10x less noise
- 10,000 samples: 100x less noise (approaching "clean")

The UI displays the current sample count and elapsed render time so the user can judge convergence.

---

## 8. Tone Mapping & Post-Processing

### Tone Mapping Pipeline

**File:** `src/shaders/wgsl/tonemap.wgsl`

The path tracer produces High Dynamic Range (HDR) radiance values that can range from 0 to thousands. Displays can only show Low Dynamic Range (LDR) values in [0, 1]. Tone mapping compresses HDR to LDR while preserving visual detail.

The pipeline applies three steps:

```
apply_tonemap(color, exposure):
    1. Exposure:  exposed = color * exposure
    2. ACES:      mapped = aces_tonemap(exposed)
    3. Gamma:     output = linear_to_srgb(mapped)
```

#### Step 1: Exposure Control

Simple multiplication by the exposure value. Higher exposure brightens the image, lower exposure darkens it. This is analogous to adjusting a physical camera's exposure time.

#### Step 2: ACES Filmic Tone Mapping

Uses Stephen Hill's fit of the Academy Color Encoding System (ACES) curve:

```
aces(x) = clamp((x * (2.51x + 0.03)) / (x * (2.43x + 0.59) + 0.14))
```

This S-shaped curve:
- Preserves dark tones (shadow detail)
- Compresses bright highlights smoothly (no harsh clipping)
- Slightly boosts mid-tone contrast (pleasing to the eye)

#### Step 3: sRGB Gamma Correction

Converts from linear light to the sRGB color space that monitors expect:

```
For each channel:
    if c <= 0.0031308:  output = c * 12.92         // Linear region
    else:               output = 1.055 * c^(1/2.4) - 0.055   // Power curve
```

This piecewise function accounts for the nonlinear response of display hardware. Without gamma correction, the image would appear too dark.

### Post-Processing Effects

**File:** `src/shaders/wgsl/post_process.wgsl`

Post-processing runs as a separate compute pass that reads from the accumulation buffer (the already tone-mapped values written by the path trace pass) and writes to the output texture. Multiple effects can be chained in user-defined order (up to 8).

The post-processing pass has its own bind group and uniform buffer containing:
```
struct PostParams {
    width: u32,
    height: u32,
    effect_count: u32,
    _pad: u32,
    effects_0_3: vec4u,    // Effect IDs for slots 0-3
    effects_4_7: vec4u,    // Effect IDs for slots 4-7
}
```

Available effects:

| Effect | Description |
|--------|-------------|
| **FXAA** | Fast Approximate Anti-Aliasing. Detects edges via luma contrast between neighboring pixels, then blurs along the dominant edge direction (horizontal or vertical). Threshold: `max(0.0312, luma * 0.125)`. |
| **Negative** | Color inversion: `1 - color` |
| **Sepia** | Warm vintage tone via a 3x3 color matrix transform |
| **Grayscale** | BT.709 luminance: `0.2126*R + 0.7152*G + 0.0722*B` |
| **Black & White** | Hard threshold grayscale: pixels above 0.5 luminance become white, below become black |
| **Oil Painting** | 7x7 box blur (radius 3) averaging, creating a painterly smoothing effect |
| **Comic** | Cel shading: quantize colors to 4 levels + Sobel edge detection for outlines |
| **Casting** | Emboss/relief effect using a 3x3 convolution kernel, shifted to [0,1] range |

---

## 9. GPU Data Layout

### Bind Groups

The path trace compute shader uses two bind groups:

**Bind Group 0: Per-Frame Data** (recreated on resize)

| Binding | Type | Resource | Description |
|---------|------|----------|-------------|
| 0 | `uniform` | `camera_buffer` | Camera position, orientation, frame index, dimensions |
| 1 | `storage (read-write)` | `accumulation_buffer` | `vec4f` per pixel, running mean of radiance |
| 2 | `storage_texture (write)` | `output_texture` | RGBA8 output for display |

**Bind Group 1: Scene Data** (recreated when scene changes)

| Binding | Type | Resource | Description |
|---------|------|----------|-------------|
| 0 | `storage (read)` | `shape_buffer` | Array of `GpuShape` (Figure) structs |
| 1 | `storage (read)` | `material_buffer` | Array of `GpuMaterial` (Material) structs |
| 2 | `storage (read)` | `bvh_node_buffer` | Flat BVH node array |
| 3 | `storage (read)` | `bvh_prim_buffer` | Primitive index indirection array |
| 4 | `storage (read)` | `light_index_buffer` | Indices of emissive shapes |
| 5 | `storage (read)` | `tex_pixels_buffer` | Packed RGBA pixels (0xAABBGGRR) |
| 6 | `storage (read)` | `tex_infos_buffer` | Per-texture width/height/offset |

**Blit Bind Group:**

| Binding | Type | Resource |
|---------|------|----------|
| 0 | `texture_2d` | `output_texture` (as sample source) |
| 1 | `sampler` | Linear filtering sampler |

**Post-Processing Bind Group:**

| Binding | Type | Resource |
|---------|------|----------|
| 0 | `uniform` | `post_params_buffer` (effect list) |
| 1 | `storage (read)` | `accumulation_buffer` |
| 2 | `storage_texture (write)` | `output_texture` |

### GPU Struct Layout

All structs use `#[repr(C)]` with `bytemuck::Pod` for direct memory mapping. They are carefully padded for GPU alignment (vec3f fields are 16-byte aligned with explicit padding).

**Camera** (uniform, 80 bytes):
```
position:     vec3f + focal_length: f32     // 16 bytes
right:        vec3f + aspect: f32           // 16 bytes
up:           vec3f + exposure: f32         // 16 bytes
forward:      vec3f + frame_index: u32      // 16 bytes
width: u32, height: u32, sample_count: u32, _pad: u32   // 16 bytes
```

**Figure** (storage, 112 bytes):
```
figure_type: u32, material_idx: u32, radius: f32, radius2: f32   // 16 bytes
position: vec3f + height: f32                                      // 16 bytes
normal: vec3f + csg_op: u32                                        // 16 bytes
rotation: vec3f + texture_scale: f32                               // 16 bytes
v0: vec3f + _pad2: f32                                             // 16 bytes
v1: vec3f + _pad3: f32                                             // 16 bytes
v2: vec3f + _pad4: f32                                             // 16 bytes
```

The Figure struct packs diverse shape parameters into a union-like layout:
- `radius`, `radius2`, `height`: Used differently per shape type (e.g., for a torus, `radius` = major radius, `radius2` = minor radius)
- `normal`: Axis direction for cylinders/cones, plane normal
- `v0`, `v1`, `v2`: Triangle vertices, or fractal parameters in `v0.xy`
- `_pad2`, `_pad3`, `_pad4`: Used by triangles to store packed half-float UV coordinates
- `csg_op`: 0 = normal shape, 1 = subtraction (CSG negative)

**Material** (storage, 48 bytes):
```
base_color: vec3f + metallic: f32    // 16 bytes
emission: vec3f + roughness: f32     // 16 bytes
emission_strength: f32, ior: f32, transmission: f32, texture_id: i32   // 16 bytes
```

**BvhNode** (storage, 32 bytes):
```
aabb_min: vec3f + left_or_prim: u32   // 16 bytes
aabb_max: vec3f + prim_count: u32     // 16 bytes
```

### Texture Atlas

**File:** `src/io/texture_atlas.rs`

All scene textures are packed into a single flat buffer of `u32` pixels (format: `0xAABBGGRR`). A separate `TextureInfo` array stores the width, height, and byte offset of each texture within the pixel buffer.

On the GPU side (`src/shaders/wgsl/textures.wgsl`), texture sampling:
1. Checks if `texture_id >= 0` (negative means no texture)
2. Wraps UV coordinates to [0, 1) using `fract()`
3. Computes the pixel index from `offset + y * width + x`
4. Unpacks the `u32` into RGB floats

This atlas approach avoids WGSL's limitation on dynamic texture array indexing while keeping all texture data in a single buffer.

### Workgroup Dispatch

Both the path trace and post-processing compute shaders use 8x8 workgroups:

```rust
dispatch_workgroups(
    ceil(width / 8),
    ceil(height / 8),
    1
)
```

Each thread handles one pixel. Threads outside the image dimensions (due to rounding) return immediately.

---

## 10. Shader Composition

### Custom WGSL Preprocessor

**File:** `src/shaders/composer.rs`

WGSL has no built-in module system. This project implements a lightweight preprocessor that enables `#import` directives within WGSL files.

#### Import Syntax

At the top of any `.wgsl` file:
```wgsl
// #import types
// #import random
// #import figures::sphere
```

The `//` prefix makes these valid WGSL comments, so the raw files remain syntactically valid WGSL even before preprocessing.

#### Module Naming

Module names are derived from file paths relative to the shader directory:
- `src/shaders/wgsl/types.wgsl` --> `types`
- `src/shaders/wgsl/figures/sphere.wgsl` --> `figures::sphere`

#### Resolution Algorithm

The `ShaderComposer` loads all `.wgsl` files from the shader directory into a `HashMap<String, String>` and resolves imports via recursive depth-first traversal with deduplication:

```
compose("path_trace"):
    resolve("path_trace", output, visited={}):
        if "path_trace" in visited: return    // Already included
        visited.add("path_trace")

        For each line in path_trace.wgsl:
            if line starts with "// #import X":
                resolve(X, output, visited)    // Recurse into dependency
            else:
                append line to body

        append body to output
```

The key properties:
1. **Dependency-first ordering**: Imports are resolved before the importing module's code is emitted, so all functions and types are declared before use.
2. **Deduplication**: Each module is included exactly once, even if imported by multiple modules. The `visited` set prevents double-inclusion.
3. **No circular dependency handling**: The visited set naturally prevents infinite loops.

#### Composition at Startup

Three shader programs are composed at application startup:

```rust
let composer = ShaderComposer::from_directory(&ShaderComposer::shader_dir())?;
let trace_source = composer.compose("path_trace")?;   // ~1200 lines after composition
let blit_source = composer.compose("blit")?;           // ~30 lines
let post_source = composer.compose("post_process")?;   // ~200 lines
```

The `path_trace` shader is the largest, pulling in types, random, utils, camera, tonemap, materials, lighting, MIS, BVH, textures, and all 17 shape intersection modules. The final composed source is a single monolithic WGSL string that is compiled by wgpu's shader compiler.

### Blit Shader

**File:** `src/shaders/wgsl/blit.wgsl`

The blit shader is a minimal render pipeline that displays the compute shader's output texture on screen. It uses the **fullscreen triangle** technique -- a single oversized triangle that covers the entire viewport, avoiding the need for a vertex buffer or quad geometry:

```
Vertex shader (3 invocations, no vertex buffer):
    vertex 0: position = (-1, -1), uv = (0, 1)
    vertex 1: position = ( 3, -1), uv = (2, 1)
    vertex 2: position = (-1,  3), uv = (0, -1)

Fragment shader:
    return textureSample(output_texture, sampler, uv)
```

The triangle extends beyond the screen edges but is clipped by the GPU's rasterizer. This is more efficient than a quad (2 triangles) because it avoids the diagonal edge where two triangles would overlap.
