use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use super::material::Material;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum FigureType {
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

impl FigureType {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figure {
    #[serde(rename = "type")]
    pub figure_type: FigureType,

    #[serde(default)]
    pub position: [f32; 3],

    /// Direction/normal (for plane, disc, cylinder axis, cone axis).
    #[serde(default = "default_normal")]
    pub normal: [f32; 3],

    /// Radius (sphere, cylinder, cone, disc, torus major, mandelbulb, julia).
    #[serde(default = "default_radius")]
    pub radius: f32,

    /// Secondary radius (torus minor radius, cone half-angle, cylinder height).
    #[serde(default)]
    pub radius2: f32,

    /// Height (cylinder, cone).
    #[serde(default)]
    pub height: f32,

    /// Rotation in degrees (Euler XYZ).
    #[serde(default)]
    pub rotation: [f32; 3],

    /// Triangle vertex 0.
    #[serde(default)]
    pub v0: [f32; 3],
    /// Triangle vertex 1.
    #[serde(default)]
    pub v1: [f32; 3],
    /// Triangle vertex 2.
    #[serde(default)]
    pub v2: [f32; 3],

    #[serde(default)]
    pub material: Material,
}

fn default_normal() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

fn default_radius() -> f32 {
    1.0
}

/// GPU-compatible figure representation. Must match the WGSL `Figure` struct layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuFigure {
    pub figure_type: u32,
    pub material_idx: u32,
    pub radius: f32,
    pub radius2: f32,

    pub position: [f32; 3],
    pub height: f32,

    pub normal: [f32; 3],
    pub _pad0: f32,

    pub rotation: [f32; 3],
    pub _pad1: f32,

    pub v0: [f32; 3],
    pub _pad2: f32,

    pub v1: [f32; 3],
    pub _pad3: f32,

    pub v2: [f32; 3],
    pub _pad4: f32,
}

impl GpuFigure {
    pub fn from_figure(fig: &Figure, material_idx: u32) -> Self {
        let normal = glam::Vec3::from(fig.normal).normalize_or_zero();
        Self {
            figure_type: fig.figure_type.as_u32(),
            material_idx,
            radius: fig.radius,
            radius2: fig.radius2,
            position: fig.position,
            height: fig.height,
            normal: normal.into(),
            _pad0: 0.0,
            rotation: fig.rotation,
            _pad1: 0.0,
            v0: fig.v0,
            _pad2: 0.0,
            v1: fig.v1,
            _pad3: 0.0,
            v2: fig.v2,
            _pad4: 0.0,
        }
    }
}
