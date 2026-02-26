// #import types

// Cylinder intersection with pre-computed axis and slab-based cap clipping.
fn intersect_cylinder(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let axis = fig.normal; // Pre-normalized axis direction
    let half_h = fig.height * 0.5;

    let oc = ray.origin - fig.position;
    let d_perp = ray.direction - dot(ray.direction, axis) * axis;
    let oc_perp = oc - dot(oc, axis) * axis;

    let a = dot(d_perp, d_perp);
    let half_b = dot(d_perp, oc_perp);
    let c = dot(oc_perp, oc_perp) - fig.radius * fig.radius;

    let discriminant = half_b * half_b - a * c;
    if discriminant < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(discriminant);
    let inv_a = 1.0 / a;

    // Try both roots
    for (var i = 0; i < 2; i++) {
        let t = select((-half_b + sqrtd) * inv_a, (-half_b - sqrtd) * inv_a, i == 0);
        if t < EPSILON {
            continue;
        }
        let p = ray.origin + ray.direction * t;
        let proj = dot(p - fig.position, axis);
        if abs(proj) <= half_h {
            hit.hit = true;
            hit.t = t;
            hit.position = p;
            hit.normal = normalize((p - fig.position) - proj * axis);
            let angle = atan2(hit.normal.z, hit.normal.x);
            hit.uv = vec2f(angle / TWO_PI + 0.5, (proj + half_h) / fig.height);
            return hit;
        }
    }

    // Check caps (top and bottom discs)
    let d_axis = dot(ray.direction, axis);
    if abs(d_axis) > EPSILON {
        for (var sign_i = 0; sign_i < 2; sign_i++) {
            let cap_sign = select(1.0, -1.0, sign_i == 1);
            let cap_center = fig.position + axis * half_h * cap_sign;
            let t = dot(cap_center - ray.origin, axis * cap_sign) / (d_axis * cap_sign);
            if t > EPSILON && t < hit.t {
                let p = ray.origin + ray.direction * t;
                let offset = p - cap_center;
                if dot(offset, offset) <= fig.radius * fig.radius {
                    hit.hit = true;
                    hit.t = t;
                    hit.position = p;
                    hit.normal = axis * cap_sign;
                    hit.uv = (offset.xz / fig.radius + 1.0) * 0.5;
                }
            }
        }
    }

    return hit;
}
