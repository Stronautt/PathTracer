// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use glam::Vec3;

use crate::scene::material::Material;
use crate::scene::shape::{Shape, ShapeType};

/// Load an OBJ model, auto-scaling so its largest dimension equals `target_size`.
/// Returns the loaded triangles positioned at `position`.
pub fn load_obj_auto_scaled(
    path: &str,
    position: [f32; 3],
    target_size: f32,
    default_material: &Material,
) -> Result<Vec<Shape>> {
    let (models, obj_materials) = tobj::load_obj(Path::new(path), &tobj::GPU_LOAD_OPTIONS)
        .with_context(|| format!("Failed to load OBJ: {path}"))?;

    // Compute extent at scale 1.0 to determine auto-scale factor.
    let mut bb_min = Vec3::splat(f32::MAX);
    let mut bb_max = Vec3::splat(f32::MIN);
    for model in &models {
        for idx in &model.mesh.indices {
            let v = read_vertex(&model.mesh.positions, *idx as usize, 1.0);
            bb_min = bb_min.min(v);
            bb_max = bb_max.max(v);
        }
    }
    let size = bb_max - bb_min;
    let extent = size.x.max(size.y).max(size.z);
    let scale = if extent > 0.0 {
        target_size / extent
    } else {
        1.0
    };

    let materials = resolve_materials(obj_materials, path);
    build_triangles(&models, &materials, path, position, scale, default_material)
}

/// Load an OBJ model with an explicit scale factor.
pub fn load_obj(
    path: &str,
    position: [f32; 3],
    scale: f32,
    default_material: &Material,
) -> Result<Vec<Shape>> {
    let (models, obj_materials) = tobj::load_obj(Path::new(path), &tobj::GPU_LOAD_OPTIONS)
        .with_context(|| format!("Failed to load OBJ: {path}"))?;

    let materials = resolve_materials(obj_materials, path);
    build_triangles(&models, &materials, path, position, scale, default_material)
}

fn resolve_materials(
    obj_materials: Result<Vec<tobj::Material>, tobj::LoadError>,
    path: &str,
) -> Vec<tobj::Material> {
    match obj_materials {
        Ok(mats) => {
            log::info!("Loaded {} materials from MTL for '{}'", mats.len(), path);
            mats
        }
        Err(e) => {
            log::warn!(
                "Failed to load MTL for '{}': {e}. Using default material.",
                path
            );
            Vec::new()
        }
    }
}

fn build_triangles(
    models: &[tobj::Model],
    materials: &[tobj::Material],
    path: &str,
    position: [f32; 3],
    scale: f32,
    default_material: &Material,
) -> Result<Vec<Shape>> {
    let obj_dir = Path::new(path).parent();

    let group_name: Arc<str> = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .into();

    // Compute bounding box at scale to find model center.
    let mut bb_min = Vec3::splat(f32::MAX);
    let mut bb_max = Vec3::splat(f32::MIN);
    for model in models {
        for idx in &model.mesh.indices {
            let v = read_vertex(&model.mesh.positions, *idx as usize, scale);
            bb_min = bb_min.min(v);
            bb_max = bb_max.max(v);
        }
    }
    let center = (bb_min + bb_max) * 0.5;
    let offset = Vec3::from(position) - center;

    let total_tris: usize = models.iter().map(|m| m.mesh.indices.len() / 3).sum();
    let mut triangles = Vec::with_capacity(total_tris);

    for model in models {
        let mesh = &model.mesh;
        let has_uvs = !mesh.texcoords.is_empty();

        let (mat, texture): (Material, Option<Arc<str>>) = if let Some(mat_id) = mesh.material_id
            && mat_id < materials.len()
        {
            let obj_mat = &materials[mat_id];
            let tex = obj_mat
                .diffuse_texture
                .as_ref()
                .map(|tex_path| Arc::from(resolve_texture_path(obj_dir, tex_path).as_str()));
            (obj_material_to_pbr(obj_mat, default_material), tex)
        } else {
            (default_material.clone(), None)
        };

        for tri in mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let v0 = read_vertex(&mesh.positions, i0, scale) + offset;
            let v1 = read_vertex(&mesh.positions, i1, scale) + offset;
            let v2 = read_vertex(&mesh.positions, i2, scale) + offset;

            let (uv0, uv1, uv2) = if has_uvs {
                (
                    read_uv(&mesh.texcoords, i0),
                    read_uv(&mesh.texcoords, i1),
                    read_uv(&mesh.texcoords, i2),
                )
            } else {
                ([0.0, 0.0], [0.0, 0.0], [0.0, 0.0])
            };

            triangles.push(Shape {
                name: Some(String::from(&*group_name)),
                shape_type: ShapeType::Triangle,
                negative: false,
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                radius: 0.0,
                radius2: 0.0,
                height: 0.0,
                rotation: [0.0, 0.0, 0.0],
                v0: v0.into(),
                v1: v1.into(),
                v2: v2.into(),
                power: 0.0,
                max_iterations: 0,
                texture: texture.as_ref().map(|t| String::from(&**t)),
                texture_scale: None,
                uv0,
                uv1,
                uv2,
                material: mat.clone(),
            });
        }
    }

    log::info!("Loaded OBJ '{}': {} triangles", path, triangles.len());
    Ok(triangles)
}

/// Convert a tobj MTL material to our PBR material.
fn obj_material_to_pbr(obj_mat: &tobj::Material, fallback: &Material) -> Material {
    let mut m = fallback.clone();

    // Kd → base_color (fallback to Ka if Kd is missing).
    if let Some(diffuse) = obj_mat.diffuse {
        m.base_color = diffuse;
    } else if let Some(ambient) = obj_mat.ambient {
        m.base_color = ambient;
    }

    // Ks → metallic (estimate from specular intensity).
    if let Some(specular) = obj_mat.specular {
        let intensity = (specular[0] + specular[1] + specular[2]) / 3.0;
        m.metallic = intensity.clamp(0.0, 1.0);
    }

    // Ns → roughness (shininess 0‒1000 → roughness 1.0‒~0.0).
    if let Some(shininess) = obj_mat.shininess {
        m.roughness = (1.0 - (shininess / 1000.0).sqrt()).clamp(0.04, 1.0);
    }

    // d (dissolve/opacity) → transmission = 1 - d.
    if let Some(dissolve) = obj_mat.dissolve
        && dissolve < 1.0
    {
        m.transmission = 1.0 - dissolve;
    }

    // Ni → IOR.
    if let Some(ior) = obj_mat.optical_density
        && ior > 0.0
    {
        m.ior = ior;
    }

    m
}

/// Resolve a texture path from an MTL file.
/// If the path already exists as-is (e.g. absolute or relative to cwd), use it directly.
/// Otherwise, resolve it relative to the OBJ file's directory.
fn resolve_texture_path(obj_dir: Option<&Path>, tex_path: &str) -> String {
    let p = Path::new(tex_path);
    if p.exists() {
        return tex_path.to_string();
    }
    if let Some(dir) = obj_dir {
        let resolved = dir.join(tex_path);
        if resolved.exists() {
            return resolved.to_string_lossy().into_owned();
        }
    }
    // Return as-is; the texture loader will report the error.
    tex_path.to_string()
}

fn read_vertex(positions: &[f32], index: usize, scale: f32) -> Vec3 {
    Vec3::new(
        positions[index * 3] * scale,
        positions[index * 3 + 1] * scale,
        positions[index * 3 + 2] * scale,
    )
}

fn read_uv(texcoords: &[f32], index: usize) -> [f32; 2] {
    let base = index * 2;
    if base + 1 < texcoords.len() {
        // Flip V: OBJ uses V=0 at bottom, but textures are stored top-to-bottom.
        [texcoords[base], 1.0 - texcoords[base + 1]]
    } else {
        [0.0, 0.0]
    }
}
