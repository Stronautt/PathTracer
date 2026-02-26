// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

use super::shape::Shape;
use crate::constants::{DEFAULT_CAMERA_POSITION, DEFAULT_EXPOSURE, DEFAULT_FOV};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub position: [f32; 3],

    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub rotation: [f32; 3],

    #[serde(default = "default_fov", skip_serializing_if = "is_default_fov")]
    pub fov: f32,

    #[serde(
        default = "default_exposure",
        skip_serializing_if = "is_default_exposure"
    )]
    pub exposure: f32,
}

fn default_fov() -> f32 {
    DEFAULT_FOV
}

fn default_exposure() -> f32 {
    DEFAULT_EXPOSURE
}

fn is_zero_vec3(v: &[f32; 3]) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}

fn is_default_fov(v: &f32) -> bool {
    *v == default_fov()
}

fn is_default_exposure(v: &f32) -> bool {
    *v == default_exposure()
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: DEFAULT_CAMERA_POSITION,
            rotation: [0.0, 0.0, 0.0],
            fov: default_fov(),
            exposure: default_exposure(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub path: String,

    #[serde(default)]
    pub position: [f32; 3],

    #[serde(default)]
    pub rotation: [f32; 3],

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default)]
    pub material: super::material::Material,
}

fn default_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    #[serde(default)]
    pub camera: CameraConfig,

    #[serde(default, alias = "figures")]
    pub shapes: Vec<Shape>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelRef>,
}

impl Scene {
    pub fn empty() -> Self {
        Self::default()
    }
}
