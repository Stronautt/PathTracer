// #import types

// Ray-AABB slab test.
fn intersect_aabb(ray: Ray, aabb_min: vec3f, aabb_max: vec3f) -> f32 {
    let inv_dir = 1.0 / ray.direction;
    let t1 = (aabb_min - ray.origin) * inv_dir;
    let t2 = (aabb_max - ray.origin) * inv_dir;
    let t_min = min(t1, t2);
    let t_max = max(t1, t2);
    let t_near = max(max(t_min.x, t_min.y), t_min.z);
    let t_far = min(min(t_max.x, t_max.y), t_max.z);
    if t_near > t_far || t_far < 0.0 {
        return MAX_T;
    }
    return max(t_near, 0.0);
}

// BVH closest-hit traversal using a stack.
fn trace_bvh(ray: Ray) -> HitRecord {
    var closest = HitRecord();
    closest.t = MAX_T;
    closest.hit = false;

    var stack: array<u32, 32>;
    var stack_ptr = 0;
    stack[0] = 0u;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr];
        let node = bvh_nodes[node_idx];

        let t_box = intersect_aabb(ray, node.aabb_min, node.aabb_max);
        if t_box >= closest.t {
            continue;
        }

        if node.prim_count > 0u {
            // Leaf node — test all primitives
            for (var i = 0u; i < node.prim_count; i++) {
                let prim_idx = bvh_prims[node.left_or_prim + i];
                let hit = intersect_figure(ray, prim_idx);
                if hit.hit && hit.t < closest.t && hit.t > EPSILON {
                    closest = hit;
                }
            }
        } else {
            // Inner node — push children, near-first for efficiency
            let left_idx = node_idx + 1u;
            let right_idx = node.left_or_prim;

            let t_left = intersect_aabb(ray, bvh_nodes[left_idx].aabb_min, bvh_nodes[left_idx].aabb_max);
            let t_right = intersect_aabb(ray, bvh_nodes[right_idx].aabb_min, bvh_nodes[right_idx].aabb_max);

            // Push far child first (so near child is processed first)
            if t_left < t_right {
                if t_right < closest.t && stack_ptr < 31 {
                    stack[stack_ptr] = right_idx;
                    stack_ptr += 1;
                }
                if t_left < closest.t && stack_ptr < 31 {
                    stack[stack_ptr] = left_idx;
                    stack_ptr += 1;
                }
            } else {
                if t_left < closest.t && stack_ptr < 31 {
                    stack[stack_ptr] = left_idx;
                    stack_ptr += 1;
                }
                if t_right < closest.t && stack_ptr < 31 {
                    stack[stack_ptr] = right_idx;
                    stack_ptr += 1;
                }
            }
        }
    }

    return closest;
}

// BVH shadow ray: any-hit early termination.
fn trace_shadow(ray: Ray, max_t: f32) -> bool {
    var stack: array<u32, 32>;
    var stack_ptr = 0;
    stack[0] = 0u;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr];
        let node = bvh_nodes[node_idx];

        let t_box = intersect_aabb(ray, node.aabb_min, node.aabb_max);
        if t_box >= max_t {
            continue;
        }

        if node.prim_count > 0u {
            for (var i = 0u; i < node.prim_count; i++) {
                let prim_idx = bvh_prims[node.left_or_prim + i];
                let hit = intersect_figure(ray, prim_idx);
                if hit.hit && hit.t > EPSILON && hit.t < max_t - EPSILON {
                    return true; // Occluded
                }
            }
        } else {
            let left_idx = node_idx + 1u;
            let right_idx = node.left_or_prim;
            if stack_ptr < 31 {
                stack[stack_ptr] = left_idx;
                stack_ptr += 1;
            }
            if stack_ptr < 31 {
                stack[stack_ptr] = right_idx;
                stack_ptr += 1;
            }
        }
    }

    return false; // Not occluded
}
