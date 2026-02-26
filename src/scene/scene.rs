use serde::{Deserialize, Serialize};

use super::figure::Figure;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    #[serde(default)]
    pub position: [f32; 3],

    #[serde(default)]
    pub rotation: [f32; 3],

    #[serde(default = "default_fov")]
    pub fov: f32,

    #[serde(default = "default_exposure")]
    pub exposure: f32,
}

fn default_fov() -> f32 {
    60.0
}

fn default_exposure() -> f32 {
    1.0
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: [0.0, 2.0, -10.0],
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

    #[serde(default)]
    pub figures: Vec<Figure>,

    #[serde(default)]
    pub models: Vec<ModelRef>,
}

impl Scene {
    pub fn empty() -> Self {
        Self::default()
    }
}
