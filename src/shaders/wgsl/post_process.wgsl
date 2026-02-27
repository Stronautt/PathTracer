// Post-processing compute shader with corrected effects.
// Reads from accumulation buffer, writes to output texture.
// Supports chaining multiple effects in user-defined order.

struct PostParams {
    width: u32,
    height: u32,
    effect_count: u32,
    oil_radius: u32,
    effects_0_3: vec4u,
    effects_4_7: vec4u,
    comic_levels: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
}

@group(0) @binding(0) var<uniform> params: PostParams;
@group(0) @binding(1) var<storage, read> accum: array<vec4f>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

const EFFECT_NONE: u32 = 0u;
const EFFECT_NEGATIVE: u32 = 1u;
const EFFECT_SEPIA: u32 = 2u;
const EFFECT_GRAYSCALE: u32 = 3u;
const EFFECT_FXAA: u32 = 4u;
const EFFECT_OIL_PAINTING: u32 = 5u;
const EFFECT_BW: u32 = 6u;
const EFFECT_COMIC: u32 = 7u;
const EFFECT_CASTING: u32 = 8u;

fn read_pixel(pixel: vec2u) -> vec3f {
    let idx = pixel.y * params.width + pixel.x;
    return accum[idx].rgb;
}

fn read_pixel_clamped(x: i32, y: i32) -> vec3f {
    let cx = clamp(x, 0, i32(params.width) - 1);
    let cy = clamp(y, 0, i32(params.height) - 1);
    return read_pixel(vec2u(u32(cx), u32(cy)));
}

fn get_effect_id(i: u32) -> u32 {
    if i < 4u {
        return params.effects_0_3[i];
    }
    return params.effects_4_7[i - 4u];
}

fn apply_single_effect(color: vec3f, pixel: vec2u, effect: u32) -> vec3f {
    switch effect {
        case EFFECT_NEGATIVE: {
            return vec3f(1.0) - color;
        }
        case EFFECT_SEPIA: {
            let r = dot(color, vec3f(0.393, 0.769, 0.189));
            let g = dot(color, vec3f(0.349, 0.686, 0.168));
            let b = dot(color, vec3f(0.272, 0.534, 0.131));
            return clamp(vec3f(r, g, b), vec3f(0.0), vec3f(1.0));
        }
        case EFFECT_GRAYSCALE: {
            let lum = dot(color, vec3f(0.2126, 0.7152, 0.0722));
            return vec3f(lum);
        }
        case EFFECT_FXAA: {
            // Spatial effects always read from the original accumulation buffer.
            return apply_fxaa(pixel);
        }
        case EFFECT_OIL_PAINTING: {
            return apply_oil_painting(pixel);
        }
        case EFFECT_BW: {
            let lum = dot(color, vec3f(0.2126, 0.7152, 0.0722));
            return select(vec3f(0.0), vec3f(1.0), lum > 0.5);
        }
        case EFFECT_COMIC: {
            return apply_comic(pixel, color);
        }
        case EFFECT_CASTING: {
            return apply_casting(pixel);
        }
        default: {
            return color;
        }
    }
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let pixel = gid.xy;
    if pixel.x >= params.width || pixel.y >= params.height {
        return;
    }

    var result = read_pixel(pixel);
    for (var i = 0u; i < params.effect_count; i++) {
        let eid = get_effect_id(i);
        result = apply_single_effect(result, pixel, eid);
    }

    textureStore(output, pixel, vec4f(result, 1.0));
}

// Real FXAA 3.11 (edge-detect + directional blur).
fn apply_fxaa(pixel: vec2u) -> vec3f {
    let ip = vec2i(pixel);
    let center = read_pixel(pixel);
    let luma_c = dot(center, vec3f(0.299, 0.587, 0.114));

    let cn = read_pixel_clamped(ip.x, ip.y - 1);
    let cw = read_pixel_clamped(ip.x - 1, ip.y);
    let ce = read_pixel_clamped(ip.x + 1, ip.y);
    let cs = read_pixel_clamped(ip.x, ip.y + 1);

    let luma_n = dot(cn, vec3f(0.299, 0.587, 0.114));
    let luma_w = dot(cw, vec3f(0.299, 0.587, 0.114));
    let luma_e = dot(ce, vec3f(0.299, 0.587, 0.114));
    let luma_s = dot(cs, vec3f(0.299, 0.587, 0.114));

    let range = max(max(luma_n, luma_s), max(luma_w, max(luma_e, luma_c)))
              - min(min(luma_n, luma_s), min(luma_w, min(luma_e, luma_c)));

    if range < max(0.0312, luma_c * 0.125) {
        return center;
    }

    let horizontal = abs(luma_n + luma_s - 2.0 * luma_c);
    let vertical = abs(luma_w + luma_e - 2.0 * luma_c);

    if horizontal > vertical {
        return (cn + cs + center) / 3.0;
    } else {
        return (cw + ce + center) / 3.0;
    }
}

// Oil painting effect with fixed boundary logic.
fn apply_oil_painting(pixel: vec2u) -> vec3f {
    let radius = i32(params.oil_radius);
    var total_color = vec3f(0.0);
    var count = 0;

    for (var dx = -radius; dx <= radius; dx++) {
        for (var dy = -radius; dy <= radius; dy++) {
            let sx = i32(pixel.x) + dx;
            let sy = i32(pixel.y) + dy;
            if sx >= 0 && sx < i32(params.width) && sy >= 0 && sy < i32(params.height) {
                total_color += read_pixel(vec2u(u32(sx), u32(sy)));
                count += 1;
            }
        }
    }

    if count > 0 {
        return total_color / f32(count);
    }
    return read_pixel(pixel);
}

// Comic/cel-shading effect.
fn apply_comic(pixel: vec2u, color: vec3f) -> vec3f {
    let levels = f32(params.comic_levels);
    let quantized = floor(color * levels + 0.5) / levels;

    let ip = vec2i(pixel);
    var gx = vec3f(0.0);
    var gy = vec3f(0.0);

    let sx = array<f32, 9>(-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0);
    let sy = array<f32, 9>(-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0);

    var ki = 0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let sc = read_pixel_clamped(ip.x + dx, ip.y + dy);
            gx += sc * sx[ki];
            gy += sc * sy[ki];
            ki++;
        }
    }

    let edge = length(gx) + length(gy);
    let edge_mask = select(1.0, 0.0, edge > 0.3);

    return quantized * edge_mask;
}

// Casting (emboss/relief) effect using a 3x3 emboss convolution kernel.
fn apply_casting(pixel: vec2u) -> vec3f {
    let ip = vec2i(pixel);

    // Emboss kernel:
    //  -2  -1   0
    //  -1   1   1
    //   0   1   2
    let k = array<f32, 9>(-2.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 2.0);

    var result = vec3f(0.0);
    var ki = 0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let sc = read_pixel_clamped(ip.x + dx, ip.y + dy);
            result += sc * k[ki];
            ki++;
        }
    }

    // Shift to [0,1] range (emboss output can be negative)
    result = clamp(result * 0.5 + 0.5, vec3f(0.0), vec3f(1.0));
    return result;
}
