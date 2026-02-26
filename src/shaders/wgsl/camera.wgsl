// #import types
// #import random

// Generate a camera ray using pre-computed basis vectors.
// Sub-pixel jitter provides built-in anti-aliasing through progressive accumulation.
fn generate_ray(cam: Camera, pixel: vec2f) -> Ray {
    // Sub-pixel jitter for AA
    let jitter = rand_vec2() - 0.5;
    let px = pixel + jitter;

    // Normalized device coordinates [-1, 1]
    let ndc_x = (2.0 * px.x / f32(cam.width) - 1.0) * cam.aspect;
    let ndc_y = 1.0 - 2.0 * px.y / f32(cam.height);

    // Ray direction from pre-computed basis vectors (no per-ray trig!)
    let dir = normalize(cam.right * ndc_x + cam.up * ndc_y + cam.forward * cam.focal_length);

    return Ray(cam.position, dir);
}
