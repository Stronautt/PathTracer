// #import types

// Disc intersection with squared distance check (no sqrt).
fn intersect_disc(ray: Ray, fig: Figure) -> HitRecord {
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

    let p = ray.origin + ray.direction * t;
    let offset = p - fig.position;
    let dist_sq = dot(offset, offset);
    let r_sq = fig.radius * fig.radius;

    if dist_sq > r_sq {
        return hit;
    }

    hit.hit = true;
    hit.t = t;
    hit.position = p;
    hit.normal = select(-fig.normal, fig.normal, denom < 0.0);
    hit.uv = (offset.xz / fig.radius + 1.0) * 0.5;

    return hit;
}
