// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

use super::shape::Shape;
use crate::constants::{
    DEFAULT_CAMERA_POSITION, DEFAULT_EXPOSURE, DEFAULT_FIREFLY_CLAMP, DEFAULT_FOV,
    DEFAULT_FRACTAL_MARCH_STEPS, DEFAULT_MAX_BOUNCES, DEFAULT_SKYBOX_BRIGHTNESS,
    DEFAULT_SKYBOX_COLOR, DEFAULT_TONE_MAPPER,
};

fn is_zero_vec3(v: &[f32; 3]) -> bool {
    *v == [0.0, 0.0, 0.0]
}

// serde requires free functions for `default =` and `skip_serializing_if =`
// attributes — const expressions are not accepted. The macro below generates
// the necessary pair for each field that has a non-zero default value:
//   `fn <getter>() -> T { CONST }`
//   `fn <pred>(v: &T) -> bool { *v == CONST }`
macro_rules! serde_default_fns {
    ($getter:ident, $pred:ident, $ty:ty, $val:expr) => {
        fn $getter() -> $ty {
            $val
        }
        fn $pred(v: &$ty) -> bool {
            *v == $val
        }
    };
}

serde_default_fns!(default_fov, is_default_fov, f32, DEFAULT_FOV);
serde_default_fns!(default_exposure, is_default_exposure, f32, DEFAULT_EXPOSURE);
serde_default_fns!(
    default_max_bounces,
    is_default_max_bounces,
    u32,
    DEFAULT_MAX_BOUNCES
);
serde_default_fns!(
    default_firefly_clamp,
    is_default_firefly_clamp,
    f32,
    DEFAULT_FIREFLY_CLAMP
);
serde_default_fns!(
    default_skybox_color,
    is_default_skybox_color,
    [f32; 3],
    DEFAULT_SKYBOX_COLOR
);
serde_default_fns!(
    default_skybox_brightness,
    is_default_skybox_brightness,
    f32,
    DEFAULT_SKYBOX_BRIGHTNESS
);
serde_default_fns!(
    default_tone_mapper,
    is_default_tone_mapper,
    u32,
    DEFAULT_TONE_MAPPER
);
serde_default_fns!(
    default_fractal_march_steps,
    is_default_fractal_march_steps,
    u32,
    DEFAULT_FRACTAL_MARCH_STEPS
);

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

    #[serde(
        default = "default_max_bounces",
        skip_serializing_if = "is_default_max_bounces"
    )]
    pub max_bounces: u32,

    #[serde(
        default = "default_firefly_clamp",
        skip_serializing_if = "is_default_firefly_clamp"
    )]
    pub firefly_clamp: f32,

    #[serde(
        default = "default_skybox_color",
        skip_serializing_if = "is_default_skybox_color"
    )]
    pub skybox_color: [f32; 3],

    #[serde(
        default = "default_skybox_brightness",
        skip_serializing_if = "is_default_skybox_brightness"
    )]
    pub skybox_brightness: f32,

    #[serde(
        default = "default_tone_mapper",
        skip_serializing_if = "is_default_tone_mapper"
    )]
    pub tone_mapper: u32,

    #[serde(
        default = "default_fractal_march_steps",
        skip_serializing_if = "is_default_fractal_march_steps"
    )]
    pub fractal_march_steps: u32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: DEFAULT_CAMERA_POSITION,
            rotation: [0.0, 0.0, 0.0],
            fov: DEFAULT_FOV,
            exposure: DEFAULT_EXPOSURE,
            max_bounces: DEFAULT_MAX_BOUNCES,
            firefly_clamp: DEFAULT_FIREFLY_CLAMP,
            skybox_color: DEFAULT_SKYBOX_COLOR,
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
            tone_mapper: DEFAULT_TONE_MAPPER,
            fractal_march_steps: DEFAULT_FRACTAL_MARCH_STEPS,
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
