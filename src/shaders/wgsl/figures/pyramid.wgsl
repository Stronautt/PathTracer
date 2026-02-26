// #import types

// Square pyramid: base at y=0, apex at y=height, base half-size = radius.
// 4 triangular faces + 1 square base.
fn intersect_pyramid(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let base = fig.position;
    let r = fig.radius;
    let h = fig.height;
    let apex = base + vec3f(0.0, h, 0.0);

    // Base quad vertices (CCW from above)
    let b0 = base + vec3f(-r, 0.0, -r);
    let b1 = base + vec3f( r, 0.0, -r);
    let b2 = base + vec3f( r, 0.0,  r);
    let b3 = base + vec3f(-r, 0.0,  r);

    // Test 4 triangular side faces
    hit = tri_test(ray, apex, b0, b1, hit);
    hit = tri_test(ray, apex, b1, b2, hit);
    hit = tri_test(ray, apex, b2, b3, hit);
    hit = tri_test(ray, apex, b3, b0, hit);

    // Test base (2 triangles forming a quad)
    hit = tri_test(ray, b0, b2, b1, hit);
    hit = tri_test(ray, b0, b3, b2, hit);

    return hit;
}

// Möller-Trumbore triangle intersection, keeping closest hit.
fn tri_test(ray: Ray, v0: vec3f, v1: vec3f, v2: vec3f, current: HitRecord) -> HitRecord {
    var hit = current;

    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = cross(ray.direction, e2);
    let a = dot(e1, h);

    if abs(a) < EPSILON {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
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
    if t > EPSILON && t < hit.t {
        hit.hit = true;
        hit.t = t;
        hit.position = ray.origin + ray.direction * t;
        hit.normal = normalize(cross(e1, e2));
        hit.uv = vec2f(u, v);
    }

    return hit;
}
