// #import types

// Regular tetrahedron centered at position, with circumradius = fig.radius.
fn intersect_tetrahedron(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let r = fig.radius;
    let pos = fig.position;

    // Regular tetrahedron vertices (circumradius = r)
    // Edge length = r * sqrt(8/3)
    let a = r * 0.942809; // sqrt(8/9)
    let b = r * 0.333333; // 1/3
    let c = r * 0.816497; // sqrt(2/3)
    let d = r * 0.666667; // 2/3

    let v0 = pos + vec3f(0.0, r, 0.0);
    let v1 = pos + vec3f(a, -b, -c * 0.5);
    let v2 = pos + vec3f(-a, -b, -c * 0.5);
    let v3 = pos + vec3f(0.0, -b, c);

    // 4 faces
    hit = tet_tri_test(ray, v0, v1, v2, hit);
    hit = tet_tri_test(ray, v0, v2, v3, hit);
    hit = tet_tri_test(ray, v0, v3, v1, hit);
    hit = tet_tri_test(ray, v1, v3, v2, hit);

    return hit;
}

fn tet_tri_test(ray: Ray, v0: vec3f, v1: vec3f, v2: vec3f, current: HitRecord) -> HitRecord {
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
