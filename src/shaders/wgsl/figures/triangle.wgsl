// #import types

// Möller-Trumbore triangle intersection (gold standard).
fn intersect_triangle(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let e1 = fig.v1 - fig.v0;
    let e2 = fig.v2 - fig.v0;
    let h = cross(ray.direction, e2);
    let a = dot(e1, h);

    if abs(a) < EPSILON {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - fig.v0;
    let u = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return hit;
    }

    let q = cross(s, e1);
    let v = f * dot(ray.direction, q);

    if v < 0.0 || u + v > 1.0 {
        return hit;
    }

    let t = f * dot(e2, q);
    if t < EPSILON {
        return hit;
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;
    hit.normal = normalize(cross(e1, e2));
    // Flip normal to face the ray
    if dot(hit.normal, ray.direction) > 0.0 {
        hit.normal = -hit.normal;
    }
    // Interpolate per-vertex UVs packed as half-floats in the padding fields.
    let t_uv0 = unpack2x16float(bitcast<u32>(fig._pad2));
    let t_uv1 = unpack2x16float(bitcast<u32>(fig._pad3));
    let t_uv2 = unpack2x16float(bitcast<u32>(fig._pad4));
    hit.uv = (1.0 - u - v) * t_uv0 + u * t_uv1 + v * t_uv2;

    return hit;
}
