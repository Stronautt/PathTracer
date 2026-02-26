// #import types

// Ray-AABB slab test. Accepts precomputed inverse direction for efficiency.
fn intersect_aabb(origin: vec3f, inv_dir: vec3f, aabb_min: vec3f, aabb_max: vec3f) -> f32 {
    let t1 = (aabb_min - origin) * inv_dir;
    let t2 = (aabb_max - origin) * inv_dir;
    let t_min = min(t1, t2);
    let t_max = max(t1, t2);
    let t_near = max(max(t_min.x, t_min.y), t_min.z);
    let t_far = min(min(t_max.x, t_max.y), t_max.z);
    if t_near > t_far || t_far < 0.0 {
        return MAX_T;
    }
    return max(t_near, 0.0);
}

// Check if a point is inside a figure's volume (for CSG subtraction).
fn is_inside_figure(p: vec3f, idx: u32) -> bool {
    let fig = figures[idx];
    switch fig.figure_type {
        case FIG_SPHERE: {
            return length(p - fig.position) < fig.radius;
        }
        case FIG_CUBE: {
            let local = abs(p - fig.position);
            let half = fig.radius;
            return local.x < half && local.y < half && local.z < half;
        }
        case FIG_CYLINDER: {
            let oc = p - fig.position;
            let proj = dot(oc, fig.normal);
            let perp = oc - proj * fig.normal;
            let half_h = fig.height * 0.5;
            return abs(proj) < half_h && dot(perp, perp) < fig.radius * fig.radius;
        }
        case FIG_CONE: {
            let apex = fig.position + fig.normal * fig.height;
            let oc = p - apex;
            let proj = dot(oc, fig.normal);
            if proj > 0.0 || proj < -fig.height {
                return false;
            }
            let tan2 = fig.radius2;
            let cos2 = 1.0 / (1.0 + tan2);
            let r2 = dot(oc, oc);
            let d = dot(oc, fig.normal);
            return d * d > cos2 * r2;
        }
        case FIG_ELLIPSOID: {
            let local = p - fig.position;
            let a = fig.radius;
            let b = fig.radius2;
            let c = fig.height;
            let safe_a = max(a, 0.001);
            let safe_b = max(b, 0.001);
            let safe_c = max(c, 0.001);
            let v = local / vec3f(safe_a, safe_b, safe_c);
            return dot(v, v) < 1.0;
        }
        default: {
            return false;
        }
    }
}

// Find the exit t for a ray through a negative figure (the far intersection).
fn find_exit_t(ray: Ray, idx: u32) -> f32 {
    let fig = figures[idx];
    switch fig.figure_type {
        case FIG_SPHERE: {
            let oc = ray.origin - fig.position;
            let half_b = dot(oc, ray.direction);
            let c = dot(oc, oc) - fig.radius * fig.radius;
            let disc = half_b * half_b - c;
            if disc < 0.0 { return MAX_T; }
            return -half_b + sqrt(disc);
        }
        case FIG_CUBE: {
            let half = vec3f(fig.radius);
            let box_min = fig.position - half;
            let box_max = fig.position + half;
            let inv_dir = 1.0 / ray.direction;
            let t1 = (box_min - ray.origin) * inv_dir;
            let t2 = (box_max - ray.origin) * inv_dir;
            let t_max = max(t1, t2);
            return min(min(t_max.x, t_max.y), t_max.z);
        }
        case FIG_CYLINDER: {
            let axis = fig.normal;
            let half_h = fig.height * 0.5;
            let oc = ray.origin - fig.position;
            let d_perp = ray.direction - dot(ray.direction, axis) * axis;
            let oc_perp = oc - dot(oc, axis) * axis;
            let a = dot(d_perp, d_perp);
            let half_b = dot(d_perp, oc_perp);
            let c = dot(oc_perp, oc_perp) - fig.radius * fig.radius;
            let disc = half_b * half_b - a * c;
            if disc < 0.0 { return MAX_T; }
            let t_far = (-half_b + sqrt(disc)) / a;
            // Also check cap exits
            let d_axis = dot(ray.direction, axis);
            if abs(d_axis) > EPSILON {
                let t_top = (half_h - dot(oc, axis)) / d_axis;
                let t_bot = (-half_h - dot(oc, axis)) / d_axis;
                let t_cap = max(t_top, t_bot);
                return min(t_far, t_cap);
            }
            return t_far;
        }
        case FIG_CONE: {
            let axis = fig.normal;
            let apex = fig.position + axis * fig.height;
            let oc = ray.origin - apex;
            let tan2 = fig.radius2;
            let cos2 = 1.0 / (1.0 + tan2);
            let d_dot_v = dot(ray.direction, axis);
            let oc_dot_v = dot(oc, axis);
            let a_c = d_dot_v * d_dot_v - cos2 * dot(ray.direction, ray.direction);
            let b_c = d_dot_v * oc_dot_v - cos2 * dot(ray.direction, oc);
            let c_c = oc_dot_v * oc_dot_v - cos2 * dot(oc, oc);
            let disc = b_c * b_c - a_c * c_c;
            if disc < 0.0 { return MAX_T; }
            return (-b_c + sqrt(disc)) / a_c;
        }
        default: {
            return MAX_T;
        }
    }
}

// Check if a hit point is inside any negative shape.
fn is_inside_any_negative(p: vec3f) -> bool {
    let num_figs = arrayLength(&figures);
    for (var i = 0u; i < num_figs; i++) {
        if figures[i].csg_op == 1u && is_inside_figure(p, i) {
            return true;
        }
    }
    return false;
}

// BVH closest-hit traversal using a stack, with CSG subtraction.
fn trace_bvh(ray: Ray) -> HitRecord {
    var closest = trace_bvh_positive(ray);

    // Apply CSG subtraction: if hit is inside a negative shape, advance past it.
    if closest.hit {
        var current_ray = ray;
        var total_t_offset = 0.0;

        for (var attempt = 0u; attempt < 8u; attempt++) {
            if !is_inside_any_negative(closest.position) {
                break;
            }

            // Find the nearest exit from all negative shapes containing the hit
            var min_exit_t = MAX_T;
            let num_figs = arrayLength(&figures);
            for (var i = 0u; i < num_figs; i++) {
                if figures[i].csg_op == 1u && is_inside_figure(closest.position, i) {
                    let exit_t = find_exit_t(current_ray, i);
                    min_exit_t = min(min_exit_t, exit_t);
                }
            }

            if min_exit_t >= MAX_T {
                break;
            }

            // Advance ray past the negative shape exit
            let advance = min_exit_t + EPSILON * 2.0;
            total_t_offset += advance;
            current_ray = Ray(
                current_ray.origin + current_ray.direction * advance,
                current_ray.direction
            );

            closest = trace_bvh_positive(current_ray);
            if closest.hit {
                closest.t += total_t_offset;
            } else {
                break;
            }
        }
    }

    return closest;
}

// Linearly test infinite shapes (planes) excluded from the BVH.
// Updates `closest` in-place if a nearer hit is found.
fn test_infinite_shapes(ray: Ray, closest: ptr<function, HitRecord>) {
    let num = arrayLength(&infinite_indices);
    for (var i = 0u; i < num; i++) {
        let prim_idx = infinite_indices[i];
        // Sentinel 0xFFFFFFFF marks the empty-buffer placeholder.
        if prim_idx == 0xFFFFFFFFu {
            continue;
        }
        // Skip negative (subtraction) shapes.
        if figures[prim_idx].csg_op == 1u {
            continue;
        }
        let hit = intersect_figure(ray, prim_idx);
        if hit.hit && hit.t > EPSILON && hit.t < (*closest).t {
            *closest = hit;
        }
    }
}

// Shadow variant: returns true if any infinite shape occludes the ray in (EPSILON, max_t).
fn test_infinite_shapes_shadow(ray: Ray, max_t: f32) -> bool {
    let num = arrayLength(&infinite_indices);
    for (var i = 0u; i < num; i++) {
        let prim_idx = infinite_indices[i];
        // Sentinel 0xFFFFFFFF marks the empty-buffer placeholder.
        if prim_idx == 0xFFFFFFFFu {
            continue;
        }
        // Skip negative (subtraction) shapes — they don't block light.
        if figures[prim_idx].csg_op == 1u {
            continue;
        }
        let hit = intersect_figure(ray, prim_idx);
        if hit.hit && hit.t > EPSILON && hit.t < max_t - EPSILON {
            if !is_inside_any_negative(hit.position) {
                return true;
            }
        }
    }
    return false;
}

// BVH traversal for non-subtracted shapes, followed by a linear test for
// infinite shapes (planes) that are excluded from the BVH.
fn trace_bvh_positive(ray: Ray) -> HitRecord {
    var closest = HitRecord();
    closest.t = MAX_T;
    closest.hit = false;

    let inv_dir = 1.0 / ray.direction;

    var stack: array<u32, 32>;
    var stack_ptr = 0;
    stack[0] = 0u;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr];
        let node = bvh_nodes[node_idx];

        let t_box = intersect_aabb(ray.origin, inv_dir, node.aabb_min, node.aabb_max);
        if t_box >= closest.t {
            continue;
        }

        if node.prim_count > 0u {
            // Leaf node — test all primitives
            for (var i = 0u; i < node.prim_count; i++) {
                let prim_idx = bvh_prims[node.left_or_prim + i];
                // Skip negative (subtraction) shapes — they don't produce hits.
                if figures[prim_idx].csg_op == 1u {
                    continue;
                }
                let hit = intersect_figure(ray, prim_idx);
                if hit.hit && hit.t < closest.t && hit.t > EPSILON {
                    closest = hit;
                }
            }
        } else {
            // Inner node — push children, near-first for efficiency
            let left_idx = node_idx + 1u;
            let right_idx = node.left_or_prim;

            let t_left = intersect_aabb(ray.origin, inv_dir, bvh_nodes[left_idx].aabb_min, bvh_nodes[left_idx].aabb_max);
            let t_right = intersect_aabb(ray.origin, inv_dir, bvh_nodes[right_idx].aabb_min, bvh_nodes[right_idx].aabb_max);

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

    test_infinite_shapes(ray, &closest);

    return closest;
}

// Shadow ray: any-hit traversal with early exit, respecting CSG subtraction.
fn trace_shadow(ray: Ray, max_t: f32) -> bool {
    let inv_dir = 1.0 / ray.direction;

    var stack: array<u32, 32>;
    var stack_ptr = 0;
    stack[0] = 0u;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr];
        let node = bvh_nodes[node_idx];

        let t_box = intersect_aabb(ray.origin, inv_dir, node.aabb_min, node.aabb_max);
        if t_box >= max_t {
            continue;
        }

        if node.prim_count > 0u {
            for (var i = 0u; i < node.prim_count; i++) {
                let prim_idx = bvh_prims[node.left_or_prim + i];
                // Skip negative shapes — they don't block light.
                if figures[prim_idx].csg_op == 1u {
                    continue;
                }
                let hit = intersect_figure(ray, prim_idx);
                if hit.hit && hit.t > EPSILON && hit.t < max_t - EPSILON {
                    // Check if this hit is inside a negative shape (CSG carved out)
                    if !is_inside_any_negative(hit.position) {
                        return true;
                    }
                }
            }
        } else {
            // Shadow rays terminate on the first hit, so near-first ordering
            // provides no benefit; push children in arbitrary order.
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

    if test_infinite_shapes_shadow(ray, max_t) {
        return true;
    }

    return false;
}
