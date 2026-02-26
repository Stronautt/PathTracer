// #import types
// #import random
// #import utils
// #import camera
// #import tonemap
// #import materials
// #import lighting
// #import mis
// #import figures::dispatch
// #import bvh

// --- Bind Group 0: Camera + Accumulation + Output ---
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read_write> accumulation: array<vec4f>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

// --- Bind Group 1: Scene Data ---
@group(1) @binding(0) var<storage, read> figures: array<Figure>;
@group(1) @binding(1) var<storage, read> materials: array<Material>;
@group(1) @binding(2) var<storage, read> bvh_nodes: array<BvhNode>;
@group(1) @binding(3) var<storage, read> bvh_prims: array<u32>;
@group(1) @binding(4) var<storage, read> light_indices: array<u32>;

const MAX_BOUNCES: u32 = 16u;
const MIN_BOUNCES_RR: u32 = 3u;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let pixel = gid.xy;
    if pixel.x >= camera.width || pixel.y >= camera.height {
        return;
    }

    // Initialize RNG per (pixel, frame)
    init_rng(pixel, camera.frame_index);

    // Generate camera ray with sub-pixel jitter
    let ray = generate_ray(camera, vec2f(f32(pixel.x), f32(pixel.y)));

    // Path trace
    let radiance = trace_path(ray);

    // Welford's progressive accumulation (numerically stable)
    let idx = pixel.y * camera.width + pixel.x;
    let prev = accumulation[idx].xyz;
    let n = max(f32(camera.sample_count), 1.0);
    let accumulated = prev + (radiance - prev) / n;
    accumulation[idx] = vec4f(accumulated, 1.0);

    // Tone map and write output
    let color = apply_tonemap(accumulated, camera.exposure);
    textureStore(output, pixel, vec4f(color, 1.0));
}

fn trace_path(initial_ray: Ray) -> vec3f {
    var ray = initial_ray;
    var throughput = vec3f(1.0);
    var radiance = vec3f(0.0);
    var specular_bounce = true;

    let num_figures = arrayLength(&figures);
    let num_lights = arrayLength(&light_indices);

    for (var bounce = 0u; bounce < MAX_BOUNCES; bounce++) {
        let hit = trace_bvh(ray);
        if !hit.hit {
            // Sky contribution
            radiance += throughput * sample_skybox(ray.direction);
            break;
        }

        let fig = figures[hit.figure_idx];
        let mat = materials[fig.material_idx];

        // Emission
        if mat.emission_strength > 0.0 {
            let le = mat.emission * mat.emission_strength;
            if specular_bounce || bounce == 0u {
                radiance += throughput * le;
            } else {
                // MIS weight for BRDF sampling hitting a light
                // (would need the BRDF pdf from the previous bounce — for simplicity,
                // use a heuristic based on the light's solid angle)
                radiance += throughput * le;
            }
            break;
        }

        let wo = -ray.direction;
        var n = hit.normal;
        // Ensure normal faces the ray
        if dot(n, wo) < 0.0 {
            n = -n;
        }

        // Glass/transmission
        if mat.transmission > 0.5 {
            let glass_sample = sample_glass(wo, n, mat);
            if length(glass_sample.direction) < 0.001 {
                break;
            }
            throughput *= glass_sample.brdf_cos;
            ray = Ray(hit.position + glass_sample.direction * EPSILON * 2.0, glass_sample.direction);
            specular_bounce = true;
            continue;
        }

        // NEE: Direct light sampling (for non-specular surfaces)
        if mat.roughness > 0.04 && num_lights > 0u {
            // Pick a random light
            let light_pick = u32(rand_f32() * f32(num_lights));
            let light_idx_safe = min(light_pick, num_lights - 1u);
            let light_fig_idx = light_indices[light_idx_safe];
            let light_fig = figures[light_fig_idx];
            let light_mat = materials[light_fig.material_idx];

            // Sample a point on the light
            let light_point = sample_sphere_light(light_fig, hit.position);
            let to_light = light_point - hit.position;
            let light_dist = length(to_light);
            let light_dir = to_light / light_dist;

            let n_dot_l = dot(n, light_dir);
            if n_dot_l > 0.0 {
                // Shadow ray
                let shadow_ray = Ray(hit.position + n * EPSILON * 2.0, light_dir);
                let occluded = trace_shadow(shadow_ray, light_dist);

                if !occluded {
                    let le = light_mat.emission * light_mat.emission_strength;

                    // Light surface normal at sampled point
                    let light_normal = normalize(light_point - light_fig.position);
                    let cos_light = abs(dot(-light_dir, light_normal));

                    // PDF conversions
                    let light_area_pdf = sphere_light_pdf(light_fig, hit.position);
                    let light_solid_pdf = area_to_solid_angle_pdf(
                        light_area_pdf, light_dist * light_dist, cos_light
                    );

                    // Evaluate BRDF
                    let brdf = eval_brdf(wo, light_dir, n, mat);

                    // MIS weight
                    let brdf_pdf_val = n_dot_l * INV_PI; // Approximate BRDF pdf
                    let w = mis_weight(light_solid_pdf * f32(num_lights), brdf_pdf_val);

                    if light_solid_pdf > 0.0 {
                        radiance += throughput * le * brdf * n_dot_l * w
                            / (light_solid_pdf * f32(num_lights));
                    }
                }
            }
        }

        // BRDF importance sampling
        let brdf_sample = sample_brdf(wo, n, mat);
        if length(brdf_sample.direction) < 0.001 || brdf_sample.pdf < EPSILON {
            break;
        }

        throughput *= brdf_sample.brdf_cos / brdf_sample.pdf;
        ray = Ray(hit.position + brdf_sample.direction * EPSILON * 2.0, brdf_sample.direction);
        specular_bounce = brdf_sample.is_specular;

        // Russian Roulette (after minimum bounces)
        if bounce >= MIN_BOUNCES_RR {
            let survival = min(max(throughput.x, max(throughput.y, throughput.z)), 0.95);
            if rand_f32() > survival {
                break;
            }
            throughput /= survival;
        }

        // Firefly clamping
        let lum = luminance(throughput);
        if lum > 100.0 {
            throughput *= 100.0 / lum;
        }
    }

    return max(radiance, vec3f(0.0));
}
