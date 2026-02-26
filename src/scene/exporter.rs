use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::scene::Scene;

pub fn save_scene(scene: &Scene, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(scene).context("Failed to serialize scene")?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write scene file: {}", path.display()))?;
    log::info!("Saved scene to {}", path.display());
    Ok(())
}
