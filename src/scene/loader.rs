// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::scene::Scene;

pub fn load_scene(path: &Path) -> Result<Scene> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read scene file: {}", path.display()))?;

    let scene: Scene = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse JSON scene file: {}", path.display()))?,
        _ => serde_yml::from_str(&contents)
            .with_context(|| format!("Failed to parse YAML scene file: {}", path.display()))?,
    };

    log::info!(
        "Loaded scene: {} shapes, {} models",
        scene.shapes.len(),
        scene.models.len()
    );

    Ok(scene)
}
