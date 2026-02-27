// Tone mapping + sRGB gamma correction.

// ACES filmic tone mapping curve (Stephen Hill's fit).
fn aces_tonemap(x: vec3f) -> vec3f {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3f(0.0), vec3f(1.0));
}

// Reinhard tone mapping.
fn reinhard_tonemap(x: vec3f) -> vec3f {
    return x / (vec3f(1.0) + x);
}

// Linear to sRGB gamma correction.
fn linear_to_srgb(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}

// Full tone mapping pipeline: exposure → tone map → sRGB.
// camera.tone_mapper: 0=ACES, 1=Reinhard, 2=None
fn apply_tonemap(color: vec3f, exposure: f32) -> vec3f {
    let exposed = color * exposure;
    var mapped: vec3f;
    switch camera.tone_mapper {
        case 1u: {
            mapped = reinhard_tonemap(exposed);
        }
        case 2u: {
            mapped = clamp(exposed, vec3f(0.0), vec3f(1.0));
        }
        default: {
            mapped = aces_tonemap(exposed);
        }
    }
    return linear_to_srgb(mapped);
}
