// #import types

// Mandelbulb SDF using trig-based triplex algebra (supports variable power).
// Reference: Inigo Quilez — https://iquilezles.org/articles/mandelbulb/
fn sdf_mandelbulb(p: vec3f, power: f32, max_iter: i32) -> f32 {
    var w = p;
    var m = dot(w, w);
    var dz = 1.0;

    for (var i = 0; i < max_iter; i++) {
        // dz = power * |w|^(power-1) * dz + 1
        dz = power * pow(m, (power - 1.0) * 0.5) * dz + 1.0;

        // w = w^power + p (triplex power via spherical coordinates)
        let r = sqrt(m);
        let b = power * acos(clamp(w.y / r, -1.0, 1.0));
        let a = power * atan2(w.x, w.z);
        let rp = pow(r, power);
        w = p + rp * vec3f(sin(b) * sin(a), cos(b), sin(b) * cos(a));

        m = dot(w, w);
        if m > 256.0 {
            break;
        }
    }

    // Hubbard-Douady distance estimate
    let r = sqrt(m);
    return 0.25 * log(m) * r / dz;
}

fn intersect_mandelbulb(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    // Fractal hyperparameters (packed in v0 by CPU)
    let power = fig.v0.x;
    let max_iter = i32(fig.v0.y);

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

    for (var i = 0u; i < camera.fractal_march_steps; i++) {
        let p = ray.origin + ray.direction * t - fig.position;
        let scaled_p = p / fig.radius;
        let d = sdf_mandelbulb(scaled_p, power, max_iter) * fig.radius;

        // Over-relaxation fallback
        if d < 0.0 && prev_d > 0.0 {
            t -= prev_d * (omega - 1.0);
            let p2 = ray.origin + ray.direction * t - fig.position;
            let d2 = sdf_mandelbulb(p2 / fig.radius, power, max_iter) * fig.radius;
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
                e.xyy * sdf_mandelbulb(local + e.xyy, power, max_iter) +
                e.yyx * sdf_mandelbulb(local + e.yyx, power, max_iter) +
                e.yxy * sdf_mandelbulb(local + e.yxy, power, max_iter) +
                e.xxx * sdf_mandelbulb(local + e.xxx, power, max_iter)
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
