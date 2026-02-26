// #import types

// Cube intersection using Ray-AABB slab test (10-12x faster than 12-triangle approach).
fn intersect_cube(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let half = vec3f(fig.radius);
    let box_min = fig.position - half;
    let box_max = fig.position + half;

    let inv_dir = 1.0 / ray.direction;
    let t1 = (box_min - ray.origin) * inv_dir;
    let t2 = (box_max - ray.origin) * inv_dir;

    let t_min = min(t1, t2);
    let t_max = max(t1, t2);

    let t_near = max(max(t_min.x, t_min.y), t_min.z);
    let t_far = min(min(t_max.x, t_max.y), t_max.z);

    if t_near > t_far || t_far < EPSILON {
        return hit;
    }

    var t = t_near;
    if t < EPSILON {
        t = t_far;
        if t < EPSILON {
            return hit;
        }
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;

    // Normal from the face that was hit (the axis with the largest component)
    let p = (hit.position - fig.position) / half;
    let abs_p = abs(p);
    if abs_p.x > abs_p.y && abs_p.x > abs_p.z {
        hit.normal = vec3f(sign(p.x), 0.0, 0.0);
    } else if abs_p.y > abs_p.z {
        hit.normal = vec3f(0.0, sign(p.y), 0.0);
    } else {
        hit.normal = vec3f(0.0, 0.0, sign(p.z));
    }

    // Box UV mapping
    if abs(hit.normal.x) > 0.5 {
        hit.uv = (p.yz + 1.0) * 0.5;
    } else if abs(hit.normal.y) > 0.5 {
        hit.uv = (p.xz + 1.0) * 0.5;
    } else {
        hit.uv = (p.xy + 1.0) * 0.5;
    }

    return hit;
}
