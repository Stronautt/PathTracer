// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

// GPU / compute
pub const WORKGROUP_SIZE: u32 = 8;

// BVH construction
pub const BVH_NUM_BINS: usize = 12;
pub const BVH_LEAF_MAX_PRIMS: usize = 4;

// AABB padding
pub const AABB_EPS: f32 = 0.0001;

// Camera defaults
pub const DEFAULT_FOV: f32 = 60.0;
pub const DEFAULT_EXPOSURE: f32 = 1.0;
pub const DEFAULT_MAX_BOUNCES: u32 = 16;
pub const DEFAULT_CAMERA_POSITION: [f32; 3] = [0.0, 2.0, -10.0];

// Render settings defaults
pub const DEFAULT_FIREFLY_CLAMP: f32 = 100.0;
pub const DEFAULT_SKYBOX_COLOR: [f32; 3] = [0.5, 0.7, 1.0];
pub const DEFAULT_SKYBOX_BRIGHTNESS: f32 = 0.3;
pub const DEFAULT_TONE_MAPPER: u32 = 0; // 0=ACES, 1=Reinhard, 2=None
pub const DEFAULT_FRACTAL_MARCH_STEPS: u32 = 256;
pub const DEFAULT_OIL_RADIUS: u32 = 3;
pub const DEFAULT_COMIC_LEVELS: u32 = 4;

// Camera controller
pub const CAMERA_DEFAULT_MOVE_SPEED: f32 = 5.0;
pub const CAMERA_SPRINT_MULTIPLIER: f32 = 3.0;
pub const CAMERA_DEFAULT_SENSITIVITY: f32 = 0.15;
pub const CAMERA_RAW_ABSOLUTE_THRESHOLD: f64 = 5000.0;
pub const CAMERA_RAW_SCALE: f32 = 0.05;
pub const CAMERA_RAW_JUMP_THRESHOLD: f32 = 500.0;
pub const CAMERA_PITCH_CLAMP: f32 = 89.0;
pub const CAMERA_SPEED_STEP: f32 = 5.0;
pub const CAMERA_SPEED_MIN: f32 = 0.5;
pub const CAMERA_SPEED_MAX: f32 = 50.0;

// Interaction / picking
// Mouse movement below this threshold (in physical pixels) is treated as a
// click-to-select rather than a drag. Compared in squared space to avoid sqrt.
pub const DRAG_THRESHOLD_PX: f32 = 5.0;

// OBJ import / model scaling
pub const MODEL_AUTO_SCALE_TARGET: f32 = 3.0;

// Accumulation buffer: vec4<f32> = 16 bytes per pixel
pub const ACCUM_BYTES_PER_PIXEL: u64 = 16;

// Window defaults
pub const DEFAULT_WINDOW_WIDTH: u32 = 1280;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 720;

// Default paths
pub const WINDOW_ICON_PATH: &str = "resources/icon.png";
pub const EXAMPLE_SCENES_DIR: &str = "resources/scenes";

// Post-process params slot counts
pub const POST_PARAMS_SIZE: usize = 16;
pub const POST_PARAMS_MAX_EFFECTS: usize = 8;

/// Resolve a data-file path: check next to the executable first, then macOS bundle, then CWD.
pub fn resolve_data_path(relative: &str) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidates = [
                // Portable archives, Windows installer, AppImage
                dir.join(relative),
                // macOS .app bundle: Contents/MacOS/../Resources/<relative>
                dir.join("../Resources").join(relative),
            ];
            for path in &candidates {
                if path.exists() {
                    return path.clone();
                }
            }
        }
    }
    PathBuf::from(relative)
}

/// Resolve a relative resource path using multiple strategies:
/// 1. Return as-is if the path already exists (e.g. `cargo run` from project root)
/// 2. Try relative to the scene file's directory
/// 3. Try via `resolve_data_path()` (next to executable / macOS bundle)
/// 4. Fall back to the original path unchanged
pub fn resolve_resource_path(scene_dir: &std::path::Path, relative: &str) -> String {
    // 1. Already reachable from CWD
    if std::path::Path::new(relative).exists() {
        return relative.to_string();
    }
    // 2. Relative to the scene file's parent directory
    let scene_relative = scene_dir.join(relative);
    if scene_relative.exists() {
        return scene_relative.to_string_lossy().into_owned();
    }
    // 3. Next to the executable / inside bundle
    let data = resolve_data_path(relative);
    if data.exists() {
        return data.to_string_lossy().into_owned();
    }
    // 4. Return unchanged — let the caller handle the missing file
    relative.to_string()
}

/// Scan the bundled example scenes directory and return sorted stem names.
pub fn discover_example_scenes() -> Vec<String> {
    let dir = resolve_data_path(EXAMPLE_SCENES_DIR);
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}
