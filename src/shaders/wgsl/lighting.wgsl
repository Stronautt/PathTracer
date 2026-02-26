// #import types

// Next Event Estimation: direct light sampling.

// Sample a point on a sphere light.
fn sample_sphere_light(light: Figure, hit_pos: vec3f) -> vec3f {
    // Uniform point on sphere surface
    let r = rand_vec2();
    let cos_theta = 1.0 - 2.0 * r.x;
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = TWO_PI * r.y;

    let local_point = vec3f(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos_theta
    ) * light.radius;

    return light.position + local_point;
}

// PDF for sampling a point on a sphere surface (uniform area).
fn sphere_light_pdf(light: Figure, hit_pos: vec3f) -> f32 {
    let area = 4.0 * PI * light.radius * light.radius;
    return 1.0 / area;
}

// Convert area PDF to solid angle PDF.
fn area_to_solid_angle_pdf(area_pdf: f32, dist_sq: f32, cos_light: f32) -> f32 {
    if cos_light <= 0.0 {
        return 0.0;
    }
    return area_pdf * dist_sq / cos_light;
}
