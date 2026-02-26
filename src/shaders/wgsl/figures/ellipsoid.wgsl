// #import types

// Ellipsoid intersection: transform ray into unit-sphere space scaled by radii.
// Radii: x = fig.radius, y = fig.height, z = fig.radius2
fn intersect_ellipsoid(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let radii = vec3f(fig.radius, max(fig.height, fig.radius), max(fig.radius2, fig.radius));
    let inv_radii = 1.0 / radii;

    // Transform ray to unit-sphere space
    let oc = (ray.origin - fig.position) * inv_radii;
    let dir = ray.direction * inv_radii;

    let a = dot(dir, dir);
    let half_b = dot(oc, dir);
    let c = dot(oc, oc) - 1.0;
    let discriminant = half_b * half_b - a * c;

    if discriminant < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(discriminant);
    let inv_a = 1.0 / a;
    var t = (-half_b - sqrtd) * inv_a;
    if t < EPSILON {
        t = (-half_b + sqrtd) * inv_a;
        if t < EPSILON {
            return hit;
        }
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;

    // Normal: gradient of (x/rx)^2 + (y/ry)^2 + (z/rz)^2 = 1
    let local = hit.position - fig.position;
    hit.normal = normalize(local * inv_radii * inv_radii);

    // Spherical UV mapping on the unit-sphere
    let unit = normalize(local * inv_radii);
    hit.uv = vec2f(
        0.5 + atan2(unit.z, unit.x) / TWO_PI,
        0.5 - asin(clamp(unit.y, -1.0, 1.0)) / PI
    );

    return hit;
}
