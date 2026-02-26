use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::scene::Scene;

pub fn load_scene(path: &Path) -> Result<Scene> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read scene file: {}", path.display()))?;
    let scene: Scene = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse scene file: {}", path.display()))?;

    log::info!(
        "Loaded scene: {} figures, {} models",
        scene.figures.len(),
        scene.models.len()
    );

    Ok(scene)
}
