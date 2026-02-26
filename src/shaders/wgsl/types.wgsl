// GPU struct definitions shared across all shaders.

struct Camera {
    position: vec3f,
    focal_length: f32,
    right: vec3f,
    aspect: f32,
    up: vec3f,
    exposure: f32,
    forward: vec3f,
    frame_index: u32,
    width: u32,
    height: u32,
    sample_count: u32,
    _pad: u32,
}

struct Figure {
    figure_type: u32,
    material_idx: u32,
    radius: f32,
    radius2: f32,
    position: vec3f,
    height: f32,
    normal: vec3f,
    _pad0: f32,
    rotation: vec3f,
    _pad1: f32,
    v0: vec3f,
    _pad2: f32,
    v1: vec3f,
    _pad3: f32,
    v2: vec3f,
    _pad4: f32,
}

struct Material {
    base_color: vec3f,
    metallic: f32,
    emission: vec3f,
    roughness: f32,
    emission_strength: f32,
    ior: f32,
    transmission: f32,
    texture_id: i32,
}

struct BvhNode {
    aabb_min: vec3f,
    left_or_prim: u32,  // inner: right child idx, leaf: first prim idx
    aabb_max: vec3f,
    prim_count: u32,     // 0 = inner node
}

struct HitRecord {
    t: f32,
    position: vec3f,
    normal: vec3f,
    uv: vec2f,
    figure_idx: u32,
    hit: bool,
}

struct Ray {
    origin: vec3f,
    direction: vec3f,
}

// Figure type constants
const FIG_SPHERE: u32 = 0u;
const FIG_PLANE: u32 = 1u;
const FIG_CUBE: u32 = 2u;
const FIG_CYLINDER: u32 = 3u;
const FIG_CONE: u32 = 4u;
const FIG_TORUS: u32 = 5u;
const FIG_DISC: u32 = 6u;
const FIG_TRIANGLE: u32 = 7u;
const FIG_SKYBOX: u32 = 8u;
const FIG_MANDELBULB: u32 = 9u;
const FIG_JULIA: u32 = 10u;
const FIG_ELLIPSOID: u32 = 11u;
const FIG_PARABOLOID: u32 = 12u;
const FIG_HYPERBOLOID: u32 = 13u;
const FIG_MEBIUS: u32 = 14u;
const FIG_PYRAMID: u32 = 15u;
const FIG_TETRAHEDRON: u32 = 16u;

const PI: f32 = 3.14159265359;
const TWO_PI: f32 = 6.28318530718;
const INV_PI: f32 = 0.31830988618;
const EPSILON: f32 = 0.0001;
const MAX_T: f32 = 1e20;
