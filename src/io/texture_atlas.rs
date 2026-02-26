use std::path::Path;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};

/// Metadata for a single texture in the atlas.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TextureInfo {
    pub width: u32,
    pub height: u32,
    /// Byte offset into the pixel buffer.
    pub offset: u32,
    pub _pad: u32,
}

/// A flat texture atlas: all textures packed into a single RGBA u32 pixel buffer (0xAABBGGRR).
pub struct TextureAtlas {
    pub pixels: Vec<u32>,
    pub infos: Vec<TextureInfo>,
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self {
            pixels: vec![0xFF808080], // slot 0: 1x1 gray fallback
            infos: vec![TextureInfo {
                width: 1,
                height: 1,
                offset: 0,
                _pad: 0,
            }],
        }
    }
}

impl TextureAtlas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a texture from disk, append it to the atlas, and return its ID.
    pub fn load_texture(&mut self, path: &Path) -> Result<usize> {
        let img = image::open(path)
            .with_context(|| format!("Failed to load texture: {}", path.display()))?
            .to_rgba8();

        let width = img.width();
        let height = img.height();
        let offset = self.pixels.len() as u32;

        let pixel_count = (width * height) as usize;
        self.pixels.reserve(pixel_count);
        self.pixels.extend(
            img.as_raw()
                .chunks_exact(4)
                .map(|c| pack_rgba(c[0], c[1], c[2], c[3])),
        );

        let id = self.infos.len();
        self.infos.push(TextureInfo {
            width,
            height,
            offset,
            _pad: 0,
        });

        log::info!(
            "Loaded texture '{}' ({}x{}) as ID {id}",
            path.display(),
            width,
            height
        );
        Ok(id)
    }
}

#[inline]
fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (u32::from(a) << 24) | (u32::from(b) << 16) | (u32::from(g) << 8) | u32::from(r)
}
