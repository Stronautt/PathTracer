// PCG hash RNG — high quality, well-distributed random numbers.
// Properly seeded per (pixel, frame) to avoid correlation artifacts.

var<private> rng_state: u32;

fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn init_rng(pixel: vec2u, frame: u32) {
    rng_state = pcg_hash(pixel.x + pixel.y * 65536u + frame * 16777259u);
}

fn rand_f32() -> f32 {
    rng_state = pcg_hash(rng_state);
    return f32(rng_state) / 4294967295.0;
}

fn rand_vec2() -> vec2f {
    return vec2f(rand_f32(), rand_f32());
}

fn rand_vec3() -> vec3f {
    return vec3f(rand_f32(), rand_f32(), rand_f32());
}
