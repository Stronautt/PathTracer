// #import types

// One-sheet hyperboloid: x^2/r^2 + z^2/r^2 - y^2/r^2 = 1, capped at ±height/2.
fn intersect_hyperboloid(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let oc = ray.origin - fig.position;
    let d = ray.direction;
    let r = fig.radius;
    let half_h = fig.height * 0.5;
    let inv_r2 = 1.0 / (r * r);

    // Quadratic: (dx^2 + dz^2 - dy^2)/r^2 * t^2 + 2*(ocx*dx + ocz*dz - ocy*dy)/r^2 * t + (ocx^2 + ocz^2 - ocy^2)/r^2 - 1 = 0
    let a = (d.x * d.x + d.z * d.z - d.y * d.y) * inv_r2;
    let b = 2.0 * (oc.x * d.x + oc.z * d.z - oc.y * d.y) * inv_r2;
    let c = (oc.x * oc.x + oc.z * oc.z - oc.y * oc.y) * inv_r2 - 1.0;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 {
        let sqrtd = sqrt(discriminant);
        let inv_2a = 0.5 / a;

        for (var i = 0; i < 2; i++) {
            let t = select((-b + sqrtd) * inv_2a, (-b - sqrtd) * inv_2a, i == 0);
            if t < EPSILON || t >= hit.t {
                continue;
            }
            let p = ray.origin + d * t;
            let local_y = p.y - fig.position.y;
            if abs(local_y) <= half_h {
                hit.hit = true;
                hit.t = t;
                hit.position = p;
                let local = p - fig.position;
                hit.normal = normalize(vec3f(2.0 * local.x, -2.0 * local.y, 2.0 * local.z) * inv_r2);
                let angle = atan2(local.z, local.x);
                hit.uv = vec2f(angle / TWO_PI + 0.5, (local_y + half_h) / fig.height);
            }
        }
    }

    // Top and bottom caps
    for (var cap = 0; cap < 2; cap++) {
        let cap_y = fig.position.y + select(-half_h, half_h, cap == 1);
        let cap_normal = select(vec3f(0.0, -1.0, 0.0), vec3f(0.0, 1.0, 0.0), cap == 1);
        if abs(d.y) > EPSILON {
            let t = (cap_y - ray.origin.y) / d.y;
            if t > EPSILON && t < hit.t {
                let p = ray.origin + d * t;
                let local = p - fig.position;
                let dist2 = local.x * local.x + local.z * local.z;
                // At cap height, radius of cross section: r * sqrt(1 + (half_h/r)^2)
                let cap_r2 = r * r * (1.0 + (half_h * half_h) * inv_r2);
                if dist2 <= cap_r2 {
                    hit.hit = true;
                    hit.t = t;
                    hit.position = p;
                    hit.normal = cap_normal;
                    hit.uv = vec2f(local.x, local.z) / sqrt(cap_r2) * 0.5 + 0.5;
                }
            }
        }
    }

    return hit;
}
