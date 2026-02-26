use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

pub fn save_screenshot(pixels: &[u8], width: u32, height: u32, path: &Path) -> Result<()> {
    let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
        .context("Failed to create image from pixel data")?;
    img.save(path)
        .with_context(|| format!("Failed to save screenshot to {}", path.display()))?;
    log::info!("Screenshot saved to {}", path.display());
    Ok(())
}

pub fn default_screenshot_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    PathBuf::from(format!("screenshot_{timestamp}.png"))
}
