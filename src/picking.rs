// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use glam::Vec3;

use crate::accel::aabb::{Aabb, shape_aabb};
use crate::accel::bvh::Bvh;
use crate::camera::camera::Camera;
use crate::scene::shape::{Shape, ShapeType};

/// Construct a world-space ray from the camera through a screen pixel.
///
/// Returns `(origin, direction)` where direction is normalised.
/// Pixel coordinates are in the same space as winit cursor positions (top-left origin).
pub fn picking_ray(
    camera: &Camera,
    pixel_x: f32,
    pixel_y: f32,
    width: u32,
    height: u32,
) -> (Vec3, Vec3) {
    let (right, up, forward) = camera.basis_vectors();
    let aspect = width as f32 / height as f32;
    let focal_length = 1.0 / (camera.fov.to_radians() * 0.5).tan();

    let ndc_x = (2.0 * pixel_x / width as f32 - 1.0) * aspect;
    let ndc_y = 1.0 - 2.0 * pixel_y / height as f32;

    let dir = (forward * focal_length + right * ndc_x + up * ndc_y).normalize();
    (camera.position, dir)
}

// ---------------------------------------------------------------------------
// Exact ray-shape intersection tests (match WGSL shader logic)
// ---------------------------------------------------------------------------

/// Return the smallest positive of two values, or `None` if both are <= 0.
fn closest_positive(t1: f32, t2: f32) -> Option<f32> {
    if t1 > 0.0 {
        Some(t1)
    } else if t2 > 0.0 {
        Some(t2)
    } else {
        None
    }
}

fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    closest_positive(-b - sqrt_d, -b + sqrt_d)
}

fn ray_plane(origin: Vec3, dir: Vec3, point: Vec3, normal: Vec3) -> Option<f32> {
    let denom = dir.dot(normal);
    if denom.abs() <= 1e-6 {
        return None;
    }
    let t = (point - origin).dot(normal) / denom;
    (t > 0.0).then_some(t)
}

fn ray_disc(origin: Vec3, dir: Vec3, center: Vec3, normal: Vec3, radius: f32) -> Option<f32> {
    let t = ray_plane(origin, dir, center, normal)?;
    let hit = origin + dir * t;
    let dist_sq = (hit - center).length_squared();
    (dist_sq <= radius * radius).then_some(t)
}

fn ray_cube(origin: Vec3, dir: Vec3, center: Vec3, half: f32) -> Option<f32> {
    let inv_dir = dir.recip();
    let box_min = center - Vec3::splat(half);
    let box_max = center + Vec3::splat(half);
    let t1 = (box_min - origin) * inv_dir;
    let t2 = (box_max - origin) * inv_dir;
    let t_enter = t1.min(t2).max_element();
    let t_exit = t1.max(t2).min_element();
    if t_enter > t_exit || t_exit < 0.0 {
        None
    } else {
        Some(if t_enter > 0.0 { t_enter } else { t_exit })
    }
}

fn ray_cylinder(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    axis: Vec3,
    radius: f32,
    height: f32,
) -> Option<f32> {
    let oc = origin - center;
    let d_along = dir.dot(axis);
    let oc_along = oc.dot(axis);
    let d_perp = dir - axis * d_along;
    let oc_perp = oc - axis * oc_along;

    let a = d_perp.dot(d_perp);
    let b = 2.0 * d_perp.dot(oc_perp);
    let c = oc_perp.dot(oc_perp) - radius * radius;

    let half_h = height * 0.5;
    let mut best: Option<f32> = None;

    // Side surface — test near root first, fall through to far root if near misses the height cap.
    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 && a.abs() > 1e-12 {
        let sqrt_d = discriminant.sqrt();
        for t in [(-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a)] {
            if t > 0.0 {
                let y = oc_along + d_along * t;
                if y.abs() <= half_h && best.is_none_or(|prev| t < prev) {
                    best = Some(t);
                    break;
                }
            }
        }
    }

    // Top and bottom caps
    if d_along.abs() > 1e-6 {
        for cap_y in [-half_h, half_h] {
            let t = (cap_y - oc_along) / d_along;
            if t > 0.0 && best.is_none_or(|prev| t < prev) {
                let hit_perp = oc_perp + d_perp * t;
                if hit_perp.length_squared() <= radius * radius {
                    best = Some(t);
                }
            }
        }
    }

    best
}

fn ray_cone(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    axis: Vec3,
    tan_sq: f32,
    height: f32,
) -> Option<f32> {
    // Base disc at `center`, apex at `center + axis * height`. `tan_sq` is tan²(half-angle).
    let apex = center + axis * height;
    let oc = origin - apex;
    let cos_sq = 1.0 / (1.0 + tan_sq);

    let d_dot_v = dir.dot(axis);
    let oc_dot_v = oc.dot(axis);
    let a = d_dot_v * d_dot_v - cos_sq * dir.dot(dir);
    let b = 2.0 * (d_dot_v * oc_dot_v - cos_sq * dir.dot(oc));
    let c = oc_dot_v * oc_dot_v - cos_sq * oc.dot(oc);

    let mut best: Option<f32> = None;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 && a.abs() > 1e-12 {
        let sqrt_d = discriminant.sqrt();
        for t in [(-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a)] {
            if t > 0.0 && best.is_none_or(|prev| t < prev) {
                let hit = origin + dir * t;
                let y = (hit - center).dot(axis);
                if (0.0..=height).contains(&y) {
                    best = Some(t);
                    break;
                }
            }
        }
    }

    // Base cap disc
    let base_radius = height * tan_sq.sqrt();
    if let Some(t) = ray_disc(origin, dir, center, -axis, base_radius)
        && best.is_none_or(|prev| t < prev)
    {
        best = Some(t);
    }

    best
}

/// Möller-Trumbore ray-triangle intersection.
fn ray_triangle(origin: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = dir.cross(e2);
    let a = e1.dot(h);
    if a.abs() < 1e-7 {
        return None;
    }
    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * e2.dot(q);
    (t > 0.0).then_some(t)
}

fn ray_ellipsoid(origin: Vec3, dir: Vec3, center: Vec3, radii: Vec3) -> Option<f32> {
    let inv_r = radii.recip();
    let oc = (origin - center) * inv_r;
    let d = dir * inv_r;
    let a = d.dot(d);
    let b = 2.0 * oc.dot(d);
    let c = oc.dot(oc) - 1.0;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    closest_positive((-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a))
}

fn ray_paraboloid(origin: Vec3, dir: Vec3, center: Vec3, radius: f32, height: f32) -> Option<f32> {
    // x² + z² = radius * y, y in [0, height]
    let oc = origin - center;
    let a = dir.x * dir.x + dir.z * dir.z;
    let b = 2.0 * (oc.x * dir.x + oc.z * dir.z) - radius * dir.y;
    let c = oc.x * oc.x + oc.z * oc.z - radius * oc.y;

    let mut best: Option<f32> = None;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 && a.abs() > 1e-12 {
        let sqrt_d = discriminant.sqrt();
        for t in [(-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a)] {
            if t > 0.0 && best.is_none_or(|prev| t < prev) {
                let y = oc.y + dir.y * t;
                if (0.0..=height).contains(&y) {
                    best = Some(t);
                    break;
                }
            }
        }
    }

    // Top cap
    let cap_r_sq = radius * height;
    if dir.y.abs() > 1e-6 {
        let t = (height - oc.y) / dir.y;
        if t > 0.0 && best.is_none_or(|prev| t < prev) {
            let hx = oc.x + dir.x * t;
            let hz = oc.z + dir.z * t;
            if hx * hx + hz * hz <= cap_r_sq {
                best = Some(t);
            }
        }
    }

    best
}

fn ray_hyperboloid(origin: Vec3, dir: Vec3, center: Vec3, radius: f32, height: f32) -> Option<f32> {
    // One-sheet: x²/r² + z²/r² - y²/r² = 1, y capped at ±height/2
    let oc = origin - center;
    let r_sq = radius * radius;
    let a = (dir.x * dir.x + dir.z * dir.z - dir.y * dir.y) / r_sq;
    let b = 2.0 * (oc.x * dir.x + oc.z * dir.z - oc.y * dir.y) / r_sq;
    let c = (oc.x * oc.x + oc.z * oc.z - oc.y * oc.y) / r_sq - 1.0;

    let half_h = height * 0.5;
    let mut best: Option<f32> = None;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 && a.abs() > 1e-12 {
        let sqrt_d = discriminant.sqrt();
        for t in [(-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a)] {
            if t > 0.0 && best.is_none_or(|prev| t < prev) {
                let y = oc.y + dir.y * t;
                if y.abs() <= half_h {
                    best = Some(t);
                    break;
                }
            }
        }
    }

    // Top/bottom caps
    let cap_r_sq = r_sq * (1.0 + (half_h / radius).powi(2));
    if dir.y.abs() > 1e-6 {
        for cap_y in [-half_h, half_h] {
            let t = (cap_y - oc.y) / dir.y;
            if t > 0.0 && best.is_none_or(|prev| t < prev) {
                let hx = oc.x + dir.x * t;
                let hz = oc.z + dir.z * t;
                if hx * hx + hz * hz <= cap_r_sq {
                    best = Some(t);
                }
            }
        }
    }

    best
}

fn ray_pyramid(origin: Vec3, dir: Vec3, center: Vec3, radius: f32, height: f32) -> Option<f32> {
    // Square base (side = 2*radius) centered at `center` lying in the xz-plane, apex at y=height.
    let apex = center + Vec3::Y * height;
    let v = [
        center + Vec3::new(-radius, 0.0, -radius),
        center + Vec3::new(radius, 0.0, -radius),
        center + Vec3::new(radius, 0.0, radius),
        center + Vec3::new(-radius, 0.0, radius),
    ];

    let mut best: Option<f32> = None;
    let mut check = |t: Option<f32>| {
        if let Some(t) = t
            && t > 0.0
            && best.is_none_or(|prev| t < prev)
        {
            best = Some(t);
        }
    };

    // 4 side faces
    check(ray_triangle(origin, dir, v[0], v[1], apex));
    check(ray_triangle(origin, dir, v[1], v[2], apex));
    check(ray_triangle(origin, dir, v[2], v[3], apex));
    check(ray_triangle(origin, dir, v[3], v[0], apex));
    // 2 base triangles
    check(ray_triangle(origin, dir, v[0], v[2], v[1]));
    check(ray_triangle(origin, dir, v[0], v[3], v[2]));

    best
}

fn ray_tetrahedron(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    // Regular tetrahedron inscribed in a sphere of the given radius.
    // Vertex coordinates are derived from the canonical unit tetrahedron scaled by `radius`.
    let sqrt_8_9 = radius * 0.942_809_04; // sqrt(8/9): base vertices x-offset
    let one_third = radius * 0.333_333_34; // 1/3: base vertices y-offset (below center)
    let sqrt_2_9 = radius * 0.471_404_5; // sqrt(2/9): back-pair x-offset
    let sqrt_2_3 = radius * 0.816_496_6; // sqrt(2/3): back-pair z-offset

    let v0 = center + Vec3::new(0.0, radius, 0.0);
    let v1 = center + Vec3::new(sqrt_8_9, -one_third, 0.0);
    let v2 = center + Vec3::new(-sqrt_2_9, -one_third, sqrt_2_3);
    let v3 = center + Vec3::new(-sqrt_2_9, -one_third, -sqrt_2_3);

    let mut best: Option<f32> = None;
    let mut check = |t: Option<f32>| {
        if let Some(t) = t
            && t > 0.0
            && best.is_none_or(|prev| t < prev)
        {
            best = Some(t);
        }
    };

    check(ray_triangle(origin, dir, v0, v1, v2));
    check(ray_triangle(origin, dir, v0, v2, v3));
    check(ray_triangle(origin, dir, v0, v3, v1));
    check(ray_triangle(origin, dir, v1, v3, v2));

    best
}

// ---------------------------------------------------------------------------
// AABB intersection (used for BVH traversal and SDF-based shape proxy)
// ---------------------------------------------------------------------------

/// Slab method AABB intersection. Returns the closest positive t, or None on miss.
fn ray_aabb(origin: Vec3, inv_dir: Vec3, aabb: &Aabb) -> Option<f32> {
    let t1 = (aabb.min - origin) * inv_dir;
    let t2 = (aabb.max - origin) * inv_dir;

    let t_enter = t1.min(t2).max_element();
    let t_exit = t1.max(t2).min_element();

    if t_enter > t_exit || t_exit < 0.0 {
        None
    } else {
        Some(if t_enter > 0.0 { t_enter } else { t_exit })
    }
}

// ---------------------------------------------------------------------------
// Per-shape intersection dispatch
// ---------------------------------------------------------------------------

/// Exact intersection test for a shape, matching WGSL shader logic.
/// Returns `Some(t)` on hit, `None` on miss.
/// SDF-based shapes (Torus, Mebius, Mandelbulb, Julia) fall back to AABB proxy.
fn intersect_shape(origin: Vec3, dir: Vec3, inv_dir: Vec3, shape: &Shape) -> Option<f32> {
    let pos = Vec3::from(shape.position);
    let normal = Vec3::from(shape.normal).normalize_or_zero();

    match shape.shape_type {
        ShapeType::Skybox => None,
        ShapeType::Plane => ray_plane(origin, dir, pos, normal),
        ShapeType::Sphere => ray_sphere(origin, dir, pos, shape.radius),
        ShapeType::Disc => ray_disc(origin, dir, pos, normal, shape.radius),
        ShapeType::Cube => ray_cube(origin, dir, pos, shape.radius),
        ShapeType::Cylinder => ray_cylinder(origin, dir, pos, normal, shape.radius, shape.height),
        ShapeType::Cone => ray_cone(origin, dir, pos, normal, shape.radius2, shape.height),
        ShapeType::Triangle => ray_triangle(
            origin,
            dir,
            Vec3::from(shape.v0),
            Vec3::from(shape.v1),
            Vec3::from(shape.v2),
        ),
        ShapeType::Ellipsoid => {
            let radii = Vec3::new(
                shape.radius,
                shape.height.max(shape.radius),
                shape.radius2.max(shape.radius),
            );
            ray_ellipsoid(origin, dir, pos, radii)
        }
        ShapeType::Paraboloid => ray_paraboloid(origin, dir, pos, shape.radius, shape.height),
        ShapeType::Hyperboloid => ray_hyperboloid(origin, dir, pos, shape.radius, shape.height),
        ShapeType::Pyramid => ray_pyramid(origin, dir, pos, shape.radius, shape.height),
        ShapeType::Tetrahedron => ray_tetrahedron(origin, dir, pos, shape.radius),
        // SDF-based shapes — AABB proxy is sufficient for picking.
        ShapeType::Torus | ShapeType::Mebius | ShapeType::Mandelbulb | ShapeType::Julia => {
            ray_aabb(origin, inv_dir, &shape_aabb(shape))
        }
    }
}

// ---------------------------------------------------------------------------
// BVH-accelerated pick
// ---------------------------------------------------------------------------

/// Returns (shape_index, t, hit_point) for the closest hit, or None.
///
/// `infinite_indices` lists global shape indices for shapes excluded from the
/// BVH (e.g. planes) that must be tested linearly after BVH traversal.
pub fn pick(
    origin: Vec3,
    dir: Vec3,
    bvh: &Bvh,
    shapes: &[Shape],
    infinite_indices: &[u32],
) -> Option<(usize, f32, Vec3)> {
    if shapes.is_empty() {
        return None;
    }

    let inv_dir = dir.recip();
    let mut closest_t = f32::INFINITY;
    let mut closest_idx: Option<usize> = None;

    // BVH traversal for finite shapes.
    if !bvh.nodes.is_empty() {
        let mut stack = Vec::with_capacity(64);
        stack.push(0u32);

        while let Some(node_idx) = stack.pop() {
            let node = &bvh.nodes[node_idx as usize];
            let node_aabb = Aabb::new(Vec3::from(node.aabb_min), Vec3::from(node.aabb_max));

            let Some(t_node) = ray_aabb(origin, inv_dir, &node_aabb) else {
                continue;
            };
            if t_node > closest_t {
                continue;
            }

            if node.prim_count > 0 {
                let first = node.left_or_prim as usize;
                for i in first..(first + node.prim_count as usize) {
                    let shape_idx = bvh.prim_indices[i] as usize;
                    let shape = &shapes[shape_idx];

                    if let Some(t) = intersect_shape(origin, dir, inv_dir, shape)
                        && t > 0.0
                        && t < closest_t
                    {
                        closest_t = t;
                        closest_idx = Some(shape_idx);
                    }
                }
            } else {
                stack.push(node.left_or_prim);
                stack.push(node_idx + 1);
            }
        }
    }

    // Linear test for infinite shapes (planes) excluded from the BVH.
    for &idx in infinite_indices {
        let shape_idx = idx as usize;
        if let Some(t) = intersect_shape(origin, dir, inv_dir, &shapes[shape_idx])
            && t > 0.0
            && t < closest_t
        {
            closest_t = t;
            closest_idx = Some(shape_idx);
        }
    }

    closest_idx.map(|idx| (idx, closest_t, origin + dir * closest_t))
}
