// #import types

// Möbius strip SDF using parametric distance estimation.
// The strip is centered at the origin with major radius R = fig.radius.
fn sdf_mebius(p: vec3f, R: f32) -> f32 {
    // Half-width of the strip
    let w = R * 0.3;

    // Find closest angle on the center circle
    let angle = atan2(p.z, p.x);

    // Center of the strip at this angle
    let cx = R * cos(angle);
    let cz = R * sin(angle);
    let center = vec3f(cx, 0.0, cz);

    // Local frame: radial direction and up
    let radial = normalize(vec3f(cos(angle), 0.0, sin(angle)));

    // The Möbius twist: the strip normal rotates by half the angle
    let half_angle = angle * 0.5;
    let strip_up = cos(half_angle) * vec3f(0.0, 1.0, 0.0) + sin(half_angle) * radial;

    // Vector from center to point
    let dp = p - center;

    // Distance along strip normal
    let v = dot(dp, strip_up);
    // Distance perpendicular (in-plane of the circle, radial)
    let u = length(dp - strip_up * v) ;

    // SDF: rectangle in (u, v) space
    let du = abs(u) - w * 0.15;
    let dv = abs(v) - w;
    return length(max(vec2f(du, dv), vec2f(0.0))) + min(max(du, dv), 0.0);
}

fn intersect_mebius(ray: Ray, fig: Figure) -> HitRecord {
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

    var t = max(-b - sqrt(disc), EPSILON);
    let max_t = -b + sqrt(disc);

    for (var i = 0; i < 256; i++) {
        let p = ray.origin + ray.direction * t - fig.position;
        let d = sdf_mebius(p, fig.radius);

        if abs(d) < EPSILON * t * 0.5 {
            hit.hit = true;
            hit.t = t;
            hit.position = ray.origin + ray.direction * t;

            // Tetrahedron normal estimation
            let e = vec2f(1.0, -1.0) * 0.5773 * EPSILON * 2.0;
            let local = hit.position - fig.position;
            hit.normal = normalize(
                e.xyy * sdf_mebius(local + e.xyy, fig.radius) +
                e.yyx * sdf_mebius(local + e.yyx, fig.radius) +
                e.yxy * sdf_mebius(local + e.yxy, fig.radius) +
                e.xxx * sdf_mebius(local + e.xxx, fig.radius)
            );

            let angle = atan2(local.z, local.x);
            hit.uv = vec2f(angle / TWO_PI + 0.5, 0.5);
            return hit;
        }

        t += d;
        if t > max_t {
            break;
        }
    }

    return hit;
}
