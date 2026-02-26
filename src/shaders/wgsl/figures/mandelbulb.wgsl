// #import types

// Mandelbulb SDF using trig-free triplex algebra (2-4x faster than trig-based).
fn sdf_mandelbulb(p: vec3f, power: f32) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = length(z);

    for (var i = 0; i < 12; i++) {
        if r > 2.0 {
            break;
        }

        let r2 = r * r;
        let r4 = r2 * r2;
        let r7 = r4 * r2 * r;

        // Trig-free triplex algebra for power 8 (Inigo Quilez method)
        // This avoids acos/atan2/sin/cos per iteration
        let x = z.x; let y = z.y; let z_c = z.z;
        let x2 = x * x; let y2 = y * y; let z2 = z_c * z_c;

        let k3 = x2 + z2;
        let k2 = inverseSqrt(k3 * k3 * k3 * k3 * k3 * k3 * k3);
        let k1 = x2 + y2 + z2;
        let k4 = x2 - y2 + z2;

        dr = r7 * 8.0 * dr + 1.0;

        let k1_sq = k1 * k1;
        let k4_sq = k4 * k4;

        // Optimized triplex power-8 formula
        let new_x = p.x + 64.0 * x * y * z_c * (x2 - z2) * k4_sq * k2 * sqrt(k3);
        let new_y = p.y + -16.0 * y2 * k3 * k4_sq * k2 + k1_sq * k1_sq;
        let new_z = p.z + -8.0 * y * k4 * (x2 * x2 - 6.0 * x2 * z2 + z2 * z2) * k2 * sqrt(k3);

        z = vec3f(new_x, new_y, new_z);
        r = length(z);
    }

    return 0.5 * log(r) * r / dr;
}

fn intersect_mandelbulb(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    // Bounding sphere check
    let oc = ray.origin - fig.position;
    let b = dot(oc, ray.direction);
    let bound_r = fig.radius * 1.5;
    let c = dot(oc, oc) - bound_r * bound_r;
    let disc = b * b - c;
    if disc < 0.0 {
        return hit;
    }

    // Over-relaxation sphere marching (Keinert 2014)
    let omega = 1.3;
    var t = max(-b - sqrt(disc), EPSILON);
    let max_t = -b + sqrt(disc);
    var prev_d = 0.0;

    for (var i = 0; i < 256; i++) {
        let p = ray.origin + ray.direction * t - fig.position;
        let scaled_p = p / fig.radius;
        let d = sdf_mandelbulb(scaled_p, 8.0) * fig.radius;

        // Over-relaxation fallback
        if d < 0.0 && prev_d > 0.0 {
            t -= prev_d * (omega - 1.0);
            let p2 = ray.origin + ray.direction * t - fig.position;
            let d2 = sdf_mandelbulb(p2 / fig.radius, 8.0) * fig.radius;
            t += d2;
            prev_d = d2;
            continue;
        }

        // Distance-relative epsilon convergence
        if abs(d) < EPSILON * t * 0.5 {
            hit.hit = true;
            hit.t = t;
            hit.position = ray.origin + ray.direction * t;

            // Tetrahedron normal (4 SDF evals instead of 6)
            let e = vec2f(1.0, -1.0) * 0.5773 * EPSILON * 2.0;
            let local = (hit.position - fig.position) / fig.radius;
            hit.normal = normalize(
                e.xyy * sdf_mandelbulb(local + e.xyy, 8.0) +
                e.yyx * sdf_mandelbulb(local + e.yyx, 8.0) +
                e.yxy * sdf_mandelbulb(local + e.yxy, 8.0) +
                e.xxx * sdf_mandelbulb(local + e.xxx, 8.0)
            );

            hit.uv = vec2f(0.0);
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
