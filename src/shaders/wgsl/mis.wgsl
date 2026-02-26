// Multiple Importance Sampling — power heuristic.

fn mis_weight(pdf_a: f32, pdf_b: f32) -> f32 {
    let a2 = pdf_a * pdf_a;
    let b2 = pdf_b * pdf_b;
    return a2 / max(a2 + b2, 1e-10);
}
