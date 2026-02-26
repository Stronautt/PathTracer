// #import types

// Plane intersection. Normal is pre-normalized at load time.
fn intersect_plane(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let denom = dot(fig.normal, ray.direction);
    if abs(denom) < EPSILON {
        return hit;
    }

    let t = dot(fig.position - ray.origin, fig.normal) / denom;
    if t < EPSILON {
        return hit;
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;
    hit.normal = select(-fig.normal, fig.normal, denom < 0.0);

    // Planar UV (world-space tiling)
    let onb = build_onb(hit.normal);
    let local = hit.position - fig.position;
    hit.uv = vec2f(dot(local, onb[0]), dot(local, onb[1])) * 0.25;

    return hit;
}
