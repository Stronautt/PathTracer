// #import types

// Skybox: direct environment map lookup on ray miss (no intersection needed).
// Returns a procedural sky gradient when no skybox texture is available.
fn sample_skybox(direction: vec3f) -> vec3f {
    // Procedural sky gradient
    let t = 0.5 * (direction.y + 1.0);
    let sky_bottom = vec3f(1.0, 1.0, 1.0);
    let sky_top = vec3f(0.5, 0.7, 1.0);
    return mix(sky_bottom, sky_top, t) * 0.3;
}

// Skybox intersection for compatibility with BVH (used when skybox is a figure).
fn intersect_skybox(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    // Large sphere intersection as fallback
    let oc = ray.origin - fig.position;
    let half_b = dot(oc, ray.direction);
    let c = dot(oc, oc) - fig.radius * fig.radius;
    let disc = half_b * half_b - c;

    if disc < 0.0 {
        return hit;
    }

    var t = -half_b + sqrt(disc); // Far intersection (we're inside)
    if t < EPSILON {
        return hit;
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;
    hit.normal = -normalize(hit.position - fig.position); // Inward-facing

    let d = normalize(hit.position - fig.position);
    hit.uv = vec2f(
        0.5 + atan2(d.z, d.x) / TWO_PI,
        0.5 - asin(clamp(d.y, -1.0, 1.0)) / PI
    );

    return hit;
}
