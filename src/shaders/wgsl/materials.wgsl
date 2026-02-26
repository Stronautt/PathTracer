// #import types
// #import random
// #import utils

// PBR Cook-Torrance / GGX microfacet BRDF with importance sampling.

struct BrdfSample {
    direction: vec3f,
    brdf_cos: vec3f,    // BRDF * cos(theta) already multiplied
    pdf: f32,
    is_specular: bool,
}

// --- GGX Distribution Functions ---

// GGX/Trowbridge-Reitz Normal Distribution Function.
fn ggx_ndf(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// Smith GGX geometry function (single direction).
fn ggx_g1(n_dot_v: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let denom = n_dot_v + sqrt(a2 + (1.0 - a2) * n_dot_v * n_dot_v);
    return 2.0 * n_dot_v / denom;
}

// Smith GGX geometry function (combined).
fn ggx_g2(n_dot_l: f32, n_dot_v: f32, alpha: f32) -> f32 {
    return ggx_g1(n_dot_l, alpha) * ggx_g1(n_dot_v, alpha);
}

// --- Importance Sampling ---

// GGX importance sampling: sample a half-vector from the GGX distribution.
fn sample_ggx_half(n: vec3f, alpha: f32) -> vec3f {
    let r = rand_vec2();
    let a2 = alpha * alpha;

    let cos_theta = sqrt((1.0 - r.y) / (1.0 + (a2 - 1.0) * r.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    let phi = TWO_PI * r.x;

    let local_h = vec3f(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
    let onb = build_onb(n);
    return normalize(onb * local_h);
}

// PDF of GGX importance-sampled half-vector, converted to solid angle of wi.
fn ggx_pdf(n_dot_h: f32, v_dot_h: f32, alpha: f32) -> f32 {
    let d = ggx_ndf(n_dot_h, alpha);
    return d * n_dot_h / (4.0 * v_dot_h);
}

// Cosine hemisphere PDF.
fn cosine_pdf(n_dot_l: f32) -> f32 {
    return n_dot_l * INV_PI;
}

// --- Full BRDF Evaluation ---

// Evaluate the full PBR BRDF for a given pair of directions.
fn eval_brdf(wo: vec3f, wi: vec3f, n: vec3f, mat: Material) -> vec3f {
    let n_dot_l = max(dot(n, wi), 0.0);
    let n_dot_v = max(dot(n, wo), 0.0);
    if n_dot_l <= 0.0 || n_dot_v <= 0.0 {
        return vec3f(0.0);
    }

    let h = normalize(wo + wi);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(wo, h), 0.0);

    let alpha = mat.roughness * mat.roughness;

    // Specular (Cook-Torrance)
    let f0 = mix(vec3f(0.04), mat.base_color, mat.metallic);
    let f = fresnel_schlick(v_dot_h, f0);
    let d = ggx_ndf(n_dot_h, alpha);
    let g = ggx_g2(n_dot_l, n_dot_v, alpha);
    let specular = (d * g * f) / max(4.0 * n_dot_l * n_dot_v, EPSILON);

    // Diffuse (Lambertian)
    let kd = (1.0 - f) * (1.0 - mat.metallic);
    let diffuse = kd * mat.base_color * INV_PI;

    return diffuse + specular;
}

// --- BRDF Sampling ---

// Sample the BRDF: choose between diffuse and specular lobe.
fn sample_brdf(wo: vec3f, n: vec3f, mat: Material) -> BrdfSample {
    let alpha = mat.roughness * mat.roughness;
    let n_dot_v = max(dot(n, wo), EPSILON);

    // Probability of sampling specular vs diffuse
    let spec_weight = mix(0.04, 1.0, mat.metallic);
    let spec_prob = max(spec_weight, 0.25); // Always give specular some probability

    var result: BrdfSample;

    if rand_f32() < spec_prob {
        // Sample specular (GGX importance sampling)
        let h = sample_ggx_half(n, alpha);
        let wi = reflect_vec(-wo, h);
        let n_dot_l = dot(n, wi);

        if n_dot_l <= 0.0 {
            result.direction = vec3f(0.0);
            result.brdf_cos = vec3f(0.0);
            result.pdf = 1.0;
            result.is_specular = true;
            return result;
        }

        let n_dot_h = max(dot(n, h), 0.0);
        let v_dot_h = max(dot(wo, h), 0.0);

        let brdf = eval_brdf(wo, wi, n, mat);
        let spec_pdf = ggx_pdf(n_dot_h, v_dot_h, alpha);
        let diff_pdf = cosine_pdf(n_dot_l);
        let pdf = spec_prob * spec_pdf + (1.0 - spec_prob) * diff_pdf;

        result.direction = wi;
        result.brdf_cos = brdf * n_dot_l;
        result.pdf = max(pdf, EPSILON);
        result.is_specular = alpha < 0.01;
    } else {
        // Sample diffuse (cosine hemisphere)
        let wi = sample_cosine_hemisphere(n);
        let n_dot_l = max(dot(n, wi), 0.0);

        if n_dot_l <= 0.0 {
            result.direction = vec3f(0.0);
            result.brdf_cos = vec3f(0.0);
            result.pdf = 1.0;
            result.is_specular = false;
            return result;
        }

        let h = normalize(wo + wi);
        let n_dot_h = max(dot(n, h), 0.0);
        let v_dot_h = max(dot(wo, h), 0.0);

        let brdf = eval_brdf(wo, wi, n, mat);
        let spec_pdf = ggx_pdf(n_dot_h, v_dot_h, alpha);
        let diff_pdf = cosine_pdf(n_dot_l);
        let pdf = spec_prob * spec_pdf + (1.0 - spec_prob) * diff_pdf;

        result.direction = wi;
        result.brdf_cos = brdf * n_dot_l;
        result.pdf = max(pdf, EPSILON);
        result.is_specular = false;
    }

    return result;
}

// Sample glass material (Fresnel-weighted reflect/refract).
fn sample_glass(wo: vec3f, n: vec3f, mat: Material) -> BrdfSample {
    var result: BrdfSample;
    result.is_specular = true;

    let entering = dot(wo, n) > 0.0;
    let face_n = select(-n, n, entering);
    let eta = select(mat.ior / 1.0, 1.0 / mat.ior, entering);

    let cos_i = abs(dot(wo, face_n));
    let fresnel = fresnel_schlick_scalar(cos_i, mat.ior);

    if rand_f32() < fresnel {
        // Reflect
        result.direction = reflect_vec(-wo, face_n);
        result.brdf_cos = mat.base_color;
        result.pdf = fresnel;
    } else {
        // Refract
        let refracted = refract_vec(-wo, face_n, eta);
        if length(refracted) < 0.001 {
            // Total internal reflection
            result.direction = reflect_vec(-wo, face_n);
            result.brdf_cos = mat.base_color;
            result.pdf = 1.0;
        } else {
            result.direction = normalize(refracted);
            result.brdf_cos = mat.base_color;
            result.pdf = 1.0 - fresnel;
        }
    }

    return result;
}
