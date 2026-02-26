// #import types

// Quaternion Julia set SDF with over-relaxation sphere marching.
fn quat_mult(a: vec4f, b: vec4f) -> vec4f {
    return vec4f(
        a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w,
        a.x * b.y + a.y * b.x + a.z * b.w - a.w * b.z,
        a.x * b.z - a.y * b.w + a.z * b.x + a.w * b.y,
        a.x * b.w + a.y * b.z - a.z * b.y + a.w * b.x
    );
}

fn sdf_julia(p: vec3f, c: vec4f, max_iter: i32) -> f32 {
    var z = vec4f(p, 0.0);
    var dz = vec4f(1.0, 0.0, 0.0, 0.0);
    var r2 = dot(z, z);

    for (var i = 0; i < max_iter; i++) {
        if r2 > 16.0 {
            break;
        }
        // dz = 2 * z * dz
        dz = 2.0 * quat_mult(z, dz);
        // z = z^2 + c
        z = quat_mult(z, z) + c;
        r2 = dot(z, z);
    }

    let r = sqrt(r2);
    let dr = length(dz);
    return 0.5 * r * log(r) / dr;
}

fn intersect_julia(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    // Julia constant (stored in rotation.xyz and radius2)
    let c = vec4f(fig.rotation, fig.radius2);
    let max_iter = i32(fig.v0.y);

    // Bounding sphere check
    let oc = ray.origin - fig.position;
    let b = dot(oc, ray.direction);
    let bound_r = fig.radius * 1.5;
    let c_sph = dot(oc, oc) - bound_r * bound_r;
    let disc = b * b - c_sph;
    if disc < 0.0 {
        return hit;
    }

    // Over-relaxation sphere marching
    let omega = 1.3;
    var t = max(-b - sqrt(disc), EPSILON);
    let max_t = -b + sqrt(disc);
    var prev_d = 0.0;

    for (var i = 0; i < 256; i++) {
        let p = ray.origin + ray.direction * t - fig.position;
        let scaled_p = p / fig.radius;
        let d = sdf_julia(scaled_p, c, max_iter) * fig.radius;

        if d < 0.0 && prev_d > 0.0 {
            t -= prev_d * (omega - 1.0);
            let p2 = ray.origin + ray.direction * t - fig.position;
            let d2 = sdf_julia(p2 / fig.radius, c, max_iter) * fig.radius;
            t += d2;
            prev_d = d2;
            continue;
        }

        if abs(d) < EPSILON * t * 0.5 {
            hit.hit = true;
            hit.t = t;
            hit.position = ray.origin + ray.direction * t;

            // Tetrahedron normal
            let e = vec2f(1.0, -1.0) * 0.5773 * EPSILON * 2.0;
            let local = (hit.position - fig.position) / fig.radius;
            hit.normal = normalize(
                e.xyy * sdf_julia(local + e.xyy, c, max_iter) +
                e.yyx * sdf_julia(local + e.yyx, c, max_iter) +
                e.yxy * sdf_julia(local + e.yxy, c, max_iter) +
                e.xxx * sdf_julia(local + e.xxx, c, max_iter)
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
