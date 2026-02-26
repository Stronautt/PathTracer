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
pub const DEFAULT_CAMERA_POSITION: [f32; 3] = [0.0, 2.0, -10.0];

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

// Post-process params slot counts
pub const POST_PARAMS_SIZE: usize = 12;
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
