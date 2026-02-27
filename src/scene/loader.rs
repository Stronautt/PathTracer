// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::scene::Scene;
use crate::constants::resolve_resource_path;

pub fn load_scene(path: &Path) -> Result<Scene> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read scene file: {}", path.display()))?;

    let mut scene: Scene = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse JSON scene file: {}", path.display()))?,
        _ => serde_yml::from_str(&contents)
            .with_context(|| format!("Failed to parse YAML scene file: {}", path.display()))?,
    };

    // Resolve relative texture / model paths so scenes work from any CWD.
    let scene_dir = path.parent().unwrap_or(Path::new("."));
    for shape in &mut scene.shapes {
        if let Some(ref tex) = shape.texture {
            shape.texture = Some(resolve_resource_path(scene_dir, tex));
        }
    }
    for model in &mut scene.models {
        model.path = resolve_resource_path(scene_dir, &model.path);
    }

    log::info!(
        "Loaded scene: {} shapes, {} models",
        scene.shapes.len(),
        scene.models.len()
    );

    Ok(scene)
}
