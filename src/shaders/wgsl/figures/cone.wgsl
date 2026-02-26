// #import types

// Cone intersection with pre-computed tan^2(half_angle).
fn intersect_cone(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let axis = fig.normal;
    let apex = fig.position + axis * fig.height;
    let tan2 = fig.radius2; // Pre-computed tan^2(half_angle) stored in radius2

    let oc = ray.origin - apex;
    let d_axis = dot(ray.direction, axis);
    let oc_axis = dot(oc, axis);

    let a = d_axis * d_axis * (1.0 + tan2) - dot(ray.direction, ray.direction) * tan2
            - d_axis * d_axis + dot(ray.direction, ray.direction);
    // Simplified:
    let a2 = d_axis * d_axis - dot(ray.direction, ray.direction) * tan2 / (1.0 + tan2);

    // Use standard cone equation
    let d_dot_v = dot(ray.direction, axis);
    let oc_dot_v = dot(oc, axis);
    let cos2 = 1.0 / (1.0 + tan2);

    let a_c = d_dot_v * d_dot_v - cos2 * dot(ray.direction, ray.direction);
    let b_c = d_dot_v * oc_dot_v - cos2 * dot(ray.direction, oc);
    let c_c = oc_dot_v * oc_dot_v - cos2 * dot(oc, oc);

    let disc = b_c * b_c - a_c * c_c;
    if disc < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(disc);
    let inv_a = 1.0 / a_c;

    for (var i = 0; i < 2; i++) {
        let t = select((-b_c + sqrtd) * inv_a, (-b_c - sqrtd) * inv_a, i == 0);
        if t < EPSILON {
            continue;
        }
        let p = ray.origin + ray.direction * t;
        let proj = dot(p - apex, axis);
        // Cone extends from apex (proj=0) downward (proj=-height)
        if proj >= -fig.height && proj <= 0.0 {
            hit.hit = true;
            hit.t = t;
            hit.position = p;
            // Cone normal
            let to_p = normalize(p - apex);
            let n_proj = dot(to_p, axis);
            hit.normal = normalize(to_p - axis * n_proj * (1.0 + tan2));
            let angle = atan2(hit.normal.z, hit.normal.x);
            hit.uv = vec2f(angle / TWO_PI + 0.5, -proj / fig.height);
            return hit;
        }
    }

    // Base cap
    let cap_center = fig.position;
    let d_axis2 = dot(ray.direction, axis);
    if abs(d_axis2) > EPSILON {
        let t = dot(cap_center - ray.origin, axis) / d_axis2;
        if t > EPSILON && t < hit.t {
            let p = ray.origin + ray.direction * t;
            let offset = p - cap_center;
            if dot(offset, offset) <= fig.radius * fig.radius {
                hit.hit = true;
                hit.t = t;
                hit.position = p;
                hit.normal = -axis;
                hit.uv = (offset.xz / fig.radius + 1.0) * 0.5;
            }
        }
    }

    return hit;
}
