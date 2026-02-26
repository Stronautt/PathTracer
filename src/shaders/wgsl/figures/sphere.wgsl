// #import types

// Sphere intersection using the half-b formulation (assumes normalized direction).
fn intersect_sphere(ray: Ray, fig: Figure) -> HitRecord {
    var hit = HitRecord();
    hit.hit = false;
    hit.t = MAX_T;

    let oc = ray.origin - fig.position;
    let half_b = dot(oc, ray.direction);
    let c = dot(oc, oc) - fig.radius * fig.radius;
    let discriminant = half_b * half_b - c;

    if discriminant < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(discriminant);
    var t = -half_b - sqrtd;
    if t < EPSILON {
        t = -half_b + sqrtd;
        if t < EPSILON {
            return hit;
        }
    }

    hit.hit = true;
    hit.t = t;
    hit.position = ray.origin + ray.direction * t;
    hit.normal = normalize(hit.position - fig.position);

    // Spherical UV mapping
    let local = normalize(hit.position - fig.position);
    hit.uv = vec2f(
        0.5 + atan2(local.z, local.x) / TWO_PI,
        0.5 - asin(clamp(local.y, -1.0, 1.0)) / PI
    );

    return hit;
}
