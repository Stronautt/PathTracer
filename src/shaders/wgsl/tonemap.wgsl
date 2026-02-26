// ACES filmic tone mapping + sRGB gamma correction.

// ACES filmic tone mapping curve (Stephen Hill's fit).
fn aces_tonemap(x: vec3f) -> vec3f {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3f(0.0), vec3f(1.0));
}

// Linear to sRGB gamma correction.
fn linear_to_srgb(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}

// Full tone mapping pipeline: exposure → ACES → sRGB.
fn apply_tonemap(color: vec3f, exposure: f32) -> vec3f {
    let exposed = color * exposure;
    let mapped = aces_tonemap(exposed);
    return linear_to_srgb(mapped);
}
