// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use super::material::Material;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum ShapeType {
    Sphere = 0,
    Plane = 1,
    Cube = 2,
    Cylinder = 3,
    Cone = 4,
    Torus = 5,
    Disc = 6,
    Triangle = 7,
    Skybox = 8,
    Mandelbulb = 9,
    Julia = 10,
    Ellipsoid = 11,
    Paraboloid = 12,
    Hyperboloid = 13,
    Mebius = 14,
    Pyramid = 15,
    Tetrahedron = 16,
}

impl ShapeType {
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sphere => "Sphere",
            Self::Plane => "Plane",
            Self::Cube => "Cube",
            Self::Cylinder => "Cylinder",
            Self::Cone => "Cone",
            Self::Torus => "Torus",
            Self::Disc => "Disc",
            Self::Triangle => "Triangle",
            Self::Skybox => "Skybox",
            Self::Mandelbulb => "Mandelbulb",
            Self::Julia => "Julia",
            Self::Ellipsoid => "Ellipsoid",
            Self::Paraboloid => "Paraboloid",
            Self::Hyperboloid => "Hyperboloid",
            Self::Mebius => "Mebius",
            Self::Pyramid => "Pyramid",
            Self::Tetrahedron => "Tetrahedron",
        }
    }

    pub const ALL: &[Self] = &[
        Self::Sphere,
        Self::Plane,
        Self::Cube,
        Self::Cylinder,
        Self::Cone,
        Self::Torus,
        Self::Disc,
        Self::Triangle,
        Self::Skybox,
        Self::Mandelbulb,
        Self::Julia,
        Self::Ellipsoid,
        Self::Paraboloid,
        Self::Hyperboloid,
        Self::Mebius,
        Self::Pyramid,
        Self::Tetrahedron,
    ];

    pub const ELEMENTARY: &[Self] = &[
        Self::Sphere,
        Self::Plane,
        Self::Cube,
        Self::Cylinder,
        Self::Cone,
        Self::Disc,
        Self::Triangle,
        Self::Pyramid,
        Self::Tetrahedron,
    ];

    pub const COMPLEX: &[Self] = &[
        Self::Torus,
        Self::Ellipsoid,
        Self::Paraboloid,
        Self::Hyperboloid,
        Self::Mebius,
        Self::Mandelbulb,
        Self::Julia,
        Self::Skybox,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shape {
    #[serde(default, skip_serializing_if = "is_empty_name")]
    pub name: Option<String>,

    #[serde(rename = "type")]
    pub shape_type: ShapeType,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub negative: bool,

    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub position: [f32; 3],

    /// Direction/normal (for plane, disc, cylinder axis, cone axis).
    #[serde(default = "default_normal", skip_serializing_if = "is_default_normal")]
    pub normal: [f32; 3],

    /// Radius (sphere, cylinder, cone, disc, torus major, mandelbulb, julia).
    #[serde(default = "default_radius", skip_serializing_if = "is_default_radius")]
    pub radius: f32,

    /// Secondary radius (torus minor radius, cone half-angle, cylinder height).
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub radius2: f32,

    /// Height (cylinder, cone).
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub height: f32,

    /// Rotation in degrees (Euler XYZ).
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub rotation: [f32; 3],

    /// Triangle vertex 0.
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub v0: [f32; 3],
    /// Triangle vertex 1.
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub v1: [f32; 3],
    /// Triangle vertex 2.
    #[serde(default, skip_serializing_if = "is_zero_vec3")]
    pub v2: [f32; 3],

    /// Fractal power (Mandelbulb only, default 8).
    #[serde(default = "default_power", skip_serializing_if = "is_default_power")]
    pub power: f32,

    /// Fractal max iterations (Mandelbulb/Julia, default 12).
    #[serde(
        default = "default_max_iterations",
        skip_serializing_if = "is_default_max_iterations"
    )]
    pub max_iterations: u32,

    /// Texture image path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,

    /// Texture UV tiling scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture_scale: Option<f32>,

    /// Per-vertex UV coordinates (for textured triangles from OBJ models).
    #[serde(default, skip_serializing)]
    pub uv0: [f32; 2],
    #[serde(default, skip_serializing)]
    pub uv1: [f32; 2],
    #[serde(default, skip_serializing)]
    pub uv2: [f32; 2],

    #[serde(default, skip_serializing_if = "Material::is_default")]
    pub material: Material,
}

fn default_normal() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

fn default_radius() -> f32 {
    1.0
}

fn default_power() -> f32 {
    8.0
}

fn default_max_iterations() -> u32 {
    12
}

fn is_empty_name(v: &Option<String>) -> bool {
    v.as_ref().is_none_or(|s| s.is_empty())
}

fn is_zero_vec3(v: &[f32; 3]) -> bool {
    v[0] == 0.0 && v[1] == 0.0 && v[2] == 0.0
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

fn is_default_normal(v: &[f32; 3]) -> bool {
    *v == default_normal()
}

fn is_default_radius(v: &f32) -> bool {
    *v == default_radius()
}

fn is_default_power(v: &f32) -> bool {
    *v == default_power()
}

fn is_default_max_iterations(v: &u32) -> bool {
    *v == default_max_iterations()
}

/// GPU-compatible shape representation. Must match the WGSL `Figure` struct layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuShape {
    pub shape_type: u32,
    pub material_idx: u32,
    pub radius: f32,
    pub radius2: f32,

    pub position: [f32; 3],
    pub height: f32,

    pub normal: [f32; 3],
    pub csg_op: u32,

    pub rotation: [f32; 3],
    pub texture_scale: f32,

    pub v0: [f32; 3],
    pub _pad2: f32,

    pub v1: [f32; 3],
    pub _pad3: f32,

    pub v2: [f32; 3],
    pub _pad4: f32,
}

impl GpuShape {
    pub fn from_shape(shape: &Shape, material_idx: u32) -> Self {
        let normal = glam::Vec3::from(shape.normal).normalize_or_zero();
        let is_fractal = matches!(shape.shape_type, ShapeType::Mandelbulb | ShapeType::Julia);
        // For fractals, pack power and max_iterations into v0 (unused by fractals otherwise).
        let v0 = if is_fractal {
            [shape.power, shape.max_iterations as f32, 0.0]
        } else {
            shape.v0
        };
        Self {
            shape_type: shape.shape_type.as_u32(),
            material_idx,
            radius: shape.radius,
            radius2: shape.radius2,
            position: shape.position,
            height: shape.height,
            normal: normal.into(),
            csg_op: u32::from(shape.negative),
            rotation: shape.rotation,
            texture_scale: shape.texture_scale.unwrap_or(1.0),
            v0,
            _pad2: pack_f16x2(shape.uv0[0], shape.uv0[1]),
            v1: shape.v1,
            _pad3: pack_f16x2(shape.uv1[0], shape.uv1[1]),
            v2: shape.v2,
            _pad4: pack_f16x2(shape.uv2[0], shape.uv2[1]),
        }
    }
}

/// Pack two f32 values into a single f32 using IEEE 754 half-float encoding.
/// Matches WGSL `pack2x16float` / `unpack2x16float` layout.
fn pack_f16x2(a: f32, b: f32) -> f32 {
    let ha = f32_to_f16_bits(a) as u32;
    let hb = f32_to_f16_bits(b) as u32;
    f32::from_bits(ha | (hb << 16))
}

/// Convert an f32 to IEEE 754 binary16 (half-float) bit pattern.
fn f32_to_f16_bits(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 16) & 0x8000;
    let exponent = ((bits >> 23) & 0xFF) as i32;
    let mantissa = bits & 0x007F_FFFF;

    if exponent == 0 {
        return sign as u16; // zero / subnormal → f16 zero
    }
    if exponent == 0xFF {
        // Inf or NaN
        return if mantissa != 0 {
            (sign | 0x7E00) as u16 // NaN
        } else {
            (sign | 0x7C00) as u16 // Inf
        };
    }

    let new_exp = exponent - 127 + 15;
    if new_exp >= 31 {
        return (sign | 0x7C00) as u16; // overflow → Inf
    }
    if new_exp <= 0 {
        // Subnormal half or underflow
        if new_exp < -10 {
            return sign as u16;
        }
        let m = (mantissa | 0x0080_0000) >> (1 - new_exp + 13);
        return (sign | m) as u16;
    }

    (sign | ((new_exp as u32) << 10) | (mantissa >> 13)) as u16
}
