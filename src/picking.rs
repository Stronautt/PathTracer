use glam::Vec3;

use crate::accel::aabb::{Aabb, figure_aabb};
use crate::accel::bvh::Bvh;
use crate::camera::camera::Camera;
use crate::scene::figure::{Figure, FigureType};

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

/// Returns (figure_index, t, hit_point) for the closest hit, or None.
pub fn pick(origin: Vec3, dir: Vec3, bvh: &Bvh, figures: &[Figure]) -> Option<(usize, f32, Vec3)> {
    if figures.is_empty() || bvh.nodes.is_empty() {
        return None;
    }

    let inv_dir = dir.recip();
    let mut closest_t = f32::INFINITY;
    let mut closest_idx: Option<usize> = None;

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
                let fig_idx = bvh.prim_indices[i] as usize;
                let fig = &figures[fig_idx];

                // Planes and skyboxes are infinite; their AABBs are meaningless for picking.
                if fig.figure_type == FigureType::Plane || fig.figure_type == FigureType::Skybox {
                    continue;
                }

                if let Some(t) = ray_aabb(origin, inv_dir, &figure_aabb(fig))
                    && t > 0.0
                    && t < closest_t
                {
                    closest_t = t;
                    closest_idx = Some(fig_idx);
                }
            }
        } else {
            // left child is node_idx + 1; right child is left_or_prim
            stack.push(node.left_or_prim);
            stack.push(node_idx + 1);
        }
    }

    closest_idx.map(|idx| (idx, closest_t, origin + dir * closest_t))
}
