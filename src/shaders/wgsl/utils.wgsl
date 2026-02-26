// #import types
// #import random

// Duff et al. 2017 branchless ONB construction.
// Builds a robust orthonormal basis from a single normal vector.
fn build_onb(n: vec3f) -> mat3x3f {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let u = vec3f(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let v = vec3f(b, s + n.y * n.y * a, -n.y);
    return mat3x3f(u, v, n);
}

// Cosine-weighted hemisphere sampling.
fn sample_cosine_hemisphere(n: vec3f) -> vec3f {
    let r = rand_vec2();
    let phi = TWO_PI * r.x;
    let cos_theta = sqrt(r.y);
    let sin_theta = sqrt(1.0 - r.y);

    let local = vec3f(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
    let onb = build_onb(n);
    return normalize(onb * local);
}

// Reflect vector about normal.
fn reflect_vec(v: vec3f, n: vec3f) -> vec3f {
    return v - 2.0 * dot(v, n) * n;
}

// Refract vector (Snell's law). Returns zero vector for total internal reflection.
fn refract_vec(v: vec3f, n: vec3f, eta: f32) -> vec3f {
    let cos_i = dot(-v, n);
    let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
    if sin2_t > 1.0 {
        return vec3f(0.0); // Total internal reflection
    }
    let cos_t = sqrt(1.0 - sin2_t);
    return eta * v + (eta * cos_i - cos_t) * n;
}

// Schlick's Fresnel approximation.
fn fresnel_schlick(cos_theta: f32, f0: vec3f) -> vec3f {
    let t = 1.0 - cos_theta;
    let t2 = t * t;
    return f0 + (1.0 - f0) * (t2 * t2 * t);
}

// Scalar Fresnel for dielectrics.
fn fresnel_schlick_scalar(cos_theta: f32, ior: f32) -> f32 {
    var r0 = (1.0 - ior) / (1.0 + ior);
    r0 = r0 * r0;
    let t = 1.0 - cos_theta;
    let t2 = t * t;
    return r0 + (1.0 - r0) * (t2 * t2 * t);
}

// Luminance (BT.709).
fn luminance(c: vec3f) -> f32 {
    return dot(c, vec3f(0.2126, 0.7152, 0.0722));
}
