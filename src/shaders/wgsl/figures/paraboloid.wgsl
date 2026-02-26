// #import types

// Paraboloid intersection: y = (x^2 + z^2) / radius, capped at height.
fn intersect_paraboloid(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let oc = ray.origin - fig.position;
    let d = ray.direction;
    let r = fig.radius;
    let h = fig.height;

    // Equation: x^2 + z^2 = r * y  →  quadratic in t
    let a = d.x * d.x + d.z * d.z;
    let b = 2.0 * (oc.x * d.x + oc.z * d.z) - r * d.y;
    let c = oc.x * oc.x + oc.z * oc.z - r * oc.y;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(discriminant);
    let inv_2a = 0.5 / a;

    for (var i = 0; i < 2; i++) {
        let t = select((-b + sqrtd) * inv_2a, (-b - sqrtd) * inv_2a, i == 0);
        if t < EPSILON {
            continue;
        }
        let p = ray.origin + d * t;
        let local_y = p.y - fig.position.y;
        if local_y >= 0.0 && local_y <= h {
            hit.hit = true;
            hit.t = t;
            hit.position = p;
            // Normal: gradient of F = x^2 + z^2 - r*y
            let local = p - fig.position;
            hit.normal = normalize(vec3f(2.0 * local.x, -r, 2.0 * local.z));
            let angle = atan2(local.z, local.x);
            hit.uv = vec2f(angle / TWO_PI + 0.5, local_y / h);
            return hit;
        }
    }

    // Top cap
    let d_y = d.y;
    if abs(d_y) > EPSILON {
        let cap_y = fig.position.y + h;
        let t = (cap_y - ray.origin.y) / d_y;
        if t > EPSILON {
            let p = ray.origin + d * t;
            let local = p - fig.position;
            let dist2 = local.x * local.x + local.z * local.z;
            if dist2 <= r * h {
                hit.hit = true;
                hit.t = t;
                hit.position = p;
                hit.normal = vec3f(0.0, 1.0, 0.0);
                hit.uv = vec2f(local.x, local.z) / sqrt(r * h) * 0.5 + 0.5;
            }
        }
    }

    return hit;
}
