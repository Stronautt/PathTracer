// #import types
// #import random

// Torus intersection via SDF sphere marching (replaces buggy quartic solver).
fn sdf_torus(p: vec3f, major_r: f32, minor_r: f32) -> f32 {
    let q = vec2f(length(p.xz) - major_r, p.y);
    return length(q) - minor_r;
}

fn intersect_torus(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let major_r = fig.radius;
    let minor_r = fig.radius2;
    let bound_r = major_r + minor_r;

    // Quick bounding sphere check
    let oc = ray.origin - fig.position;
    let b = dot(oc, ray.direction);
    let c = dot(oc, oc) - (bound_r + minor_r) * (bound_r + minor_r);
    if b * b - c < 0.0 {
        return hit;
    }

    // Sphere marching with over-relaxation
    let omega = 1.4;
    var t = max(-b - sqrt(max(b * b - c, 0.0)), EPSILON);
    let max_t = min(-b + sqrt(max(b * b - c, 0.0)), MAX_T);
    var prev_d = 0.0;

    for (var i = 0; i < 128; i++) {
        let p = ray.origin + ray.direction * t - fig.position;
        let d = sdf_torus(p, major_r, minor_r);

        // Over-relaxation: step by omega * d, but fall back if overstepped
        if d < 0.0 && prev_d > 0.0 {
            t -= prev_d * (omega - 1.0);
            let p2 = ray.origin + ray.direction * t - fig.position;
            let d2 = sdf_torus(p2, major_r, minor_r);
            t += d2;
            prev_d = d2;
            continue;
        }

        // Distance-relative convergence test
        if abs(d) < EPSILON * t {
            hit.hit = true;
            hit.t = t;
            hit.position = ray.origin + ray.direction * t;

            // Tetrahedron normal (4 SDF evals instead of 6)
            let e = vec2f(1.0, -1.0) * 0.5773 * EPSILON;
            let local = hit.position - fig.position;
            hit.normal = normalize(
                e.xyy * sdf_torus(local + e.xyy, major_r, minor_r) +
                e.yyx * sdf_torus(local + e.yyx, major_r, minor_r) +
                e.yxy * sdf_torus(local + e.yxy, major_r, minor_r) +
                e.xxx * sdf_torus(local + e.xxx, major_r, minor_r)
            );

            // Torus UV
            let angle_major = atan2(local.z, local.x);
            let proj = vec2f(length(local.xz) - major_r, local.y);
            let angle_minor = atan2(proj.y, proj.x);
            hit.uv = vec2f(angle_major / TWO_PI + 0.5, angle_minor / TWO_PI + 0.5);

            return hit;
        }

        t += d * omega;
        prev_d = d;

        if t > max_t {
            break;
        }
    }

    return hit;
}
