// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// PBR metallic-roughness material (Cook-Torrance / GGX).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material {
    #[serde(
        default = "default_base_color",
        skip_serializing_if = "is_default_base_color"
    )]
    pub base_color: [f32; 3],

    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub metallic: f32,

    #[serde(
        default = "default_roughness",
        skip_serializing_if = "is_default_roughness"
    )]
    pub roughness: f32,

    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub emission: [f32; 3],

    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub emission_strength: f32,

    #[serde(default = "default_ior", skip_serializing_if = "is_default_ior")]
    pub ior: f32,

    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub transmission: f32,

    #[serde(default = "default_no_texture", skip_serializing_if = "is_no_texture")]
    pub texture_id: i32,
}

fn default_base_color() -> [f32; 3] {
    [0.8, 0.8, 0.8]
}

fn default_roughness() -> f32 {
    0.5
}

fn default_ior() -> f32 {
    1.5
}

fn default_no_texture() -> i32 {
    -1
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

fn is_zero_vec3(v: &[f32; 3]) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}

fn is_default_base_color(v: &[f32; 3]) -> bool {
    *v == default_base_color()
}

fn is_default_roughness(v: &f32) -> bool {
    *v == default_roughness()
}

fn is_default_ior(v: &f32) -> bool {
    *v == default_ior()
}

fn is_no_texture(v: &i32) -> bool {
    *v == default_no_texture()
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: default_base_color(),
            metallic: 0.0,
            roughness: default_roughness(),
            emission: [0.0; 3],
            emission_strength: 0.0,
            ior: default_ior(),
            transmission: 0.0,
            texture_id: default_no_texture(),
        }
    }
}

impl Material {
    pub fn is_emissive(&self) -> bool {
        self.emission_strength > 0.0
            && (self.emission[0] > 0.0 || self.emission[1] > 0.0 || self.emission[2] > 0.0)
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// GPU-compatible material representation. Must match the WGSL `Material` struct layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuMaterial {
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub emission: [f32; 3],
    pub roughness: f32,
    pub emission_strength: f32,
    pub ior: f32,
    pub transmission: f32,
    pub texture_id: i32,
}

impl From<&Material> for GpuMaterial {
    fn from(mat: &Material) -> Self {
        Self {
            base_color: mat.base_color,
            metallic: mat.metallic,
            emission: mat.emission,
            roughness: mat.roughness.max(0.04), // clamp to avoid singularity in GGX
            emission_strength: mat.emission_strength,
            ior: mat.ior,
            transmission: mat.transmission,
            texture_id: mat.texture_id,
        }
    }
}
