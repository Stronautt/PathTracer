use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;

use crate::scene::figure::{Figure, FigureType};
use crate::scene::material::Material;

pub fn load_obj(
    path: &str,
    position: [f32; 3],
    scale: f32,
    default_material: &Material,
) -> Result<Vec<Figure>> {
    let (models, _materials) = tobj::load_obj(Path::new(path), &tobj::GPU_LOAD_OPTIONS)
        .with_context(|| format!("Failed to load OBJ: {path}"))?;

    let offset = Vec3::from(position);
    let mut triangles = Vec::new();

    for model in &models {
        let mesh = &model.mesh;
        for tri in mesh.indices.chunks_exact(3) {
            let v0 = read_vertex(&mesh.positions, tri[0] as usize, scale) + offset;
            let v1 = read_vertex(&mesh.positions, tri[1] as usize, scale) + offset;
            let v2 = read_vertex(&mesh.positions, tri[2] as usize, scale) + offset;

            triangles.push(Figure {
                figure_type: FigureType::Triangle,
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                radius: 0.0,
                radius2: 0.0,
                height: 0.0,
                rotation: [0.0, 0.0, 0.0],
                v0: v0.into(),
                v1: v1.into(),
                v2: v2.into(),
                material: default_material.clone(),
            });
        }
    }

    log::info!("Loaded OBJ '{}': {} triangles", path, triangles.len());
    Ok(triangles)
}

fn read_vertex(positions: &[f32], index: usize, scale: f32) -> Vec3 {
    Vec3::new(
        positions[index * 3] * scale,
        positions[index * 3 + 1] * scale,
        positions[index * 3 + 2] * scale,
    )
}
