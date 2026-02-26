// #import types

struct TextureInfo {
    width: u32,
    height: u32,
    offset: u32,
    _pad: u32,
}

@group(1) @binding(5) var<storage, read> tex_pixels: array<u32>;
@group(1) @binding(6) var<storage, read> tex_infos: array<TextureInfo>;

fn sample_texture(texture_id: i32, uv: vec2f) -> vec3f {
    if texture_id < 0 {
        return vec3f(1.0);  // no texture — white (multiplied with base_color)
    }
    let info = tex_infos[u32(texture_id)];
    // Wrap UV to [0, 1)
    let wrapped = fract(uv);
    let px = clamp(u32(wrapped.x * f32(info.width)), 0u, info.width - 1u);
    let py = clamp(u32(wrapped.y * f32(info.height)), 0u, info.height - 1u);
    let idx = info.offset + py * info.width + px;
    let packed = tex_pixels[idx];
    // Unpack 0xAABBGGRR
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    return vec3f(r, g, b);
}
