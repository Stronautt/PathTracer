use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::scene::figure::{Figure, FigureType};

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_point(p: Vec3) -> Self {
        Self { min: p, max: p }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn expand(self, p: Vec3) -> Self {
        Self {
            min: self.min.min(p),
            max: self.max.max(p),
        }
    }

    /// Surface area used for the SAH cost metric.
    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the index of the longest axis (0=x, 1=y, 2=z).
    pub fn longest_axis(&self) -> usize {
        let d = self.max - self.min;
        if d.x > d.y && d.x > d.z {
            0
        } else if d.y > d.z {
            1
        } else {
            2
        }
    }

    /// Expands any axis thinner than `eps` by `eps` on each side to avoid
    /// degenerate zero-width slabs during ray-slab intersection.
    pub fn pad(self) -> Self {
        const EPS: f32 = 0.0001;
        self.pad_axis(0, EPS).pad_axis(1, EPS).pad_axis(2, EPS)
    }

    fn pad_axis(mut self, axis: usize, eps: f32) -> Self {
        if self.max[axis] - self.min[axis] < eps {
            self.min[axis] -= eps;
            self.max[axis] += eps;
        }
        self
    }
}

/// GPU-compatible AABB used inside `GpuBvhNode`. Padded for 16-byte alignment.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuAabb {
    pub min: [f32; 3],
    pub _pad0: f32,
    pub max: [f32; 3],
    pub _pad1: f32,
}

impl From<&Aabb> for GpuAabb {
    fn from(aabb: &Aabb) -> Self {
        Self {
            min: aabb.min.into(),
            _pad0: 0.0,
            max: aabb.max.into(),
            _pad1: 0.0,
        }
    }
}

pub fn figure_aabb(fig: &Figure) -> Aabb {
    let pos = Vec3::from(fig.position);

    match fig.figure_type {
        FigureType::Sphere => {
            let r = Vec3::splat(fig.radius);
            Aabb::new(pos - r, pos + r)
        }
        FigureType::Cube => {
            let half = Vec3::splat(fig.radius);
            Aabb::new(pos - half, pos + half)
        }
        FigureType::Cylinder => {
            let extent = Vec3::new(fig.radius, fig.height * 0.5, fig.radius);
            Aabb::new(pos - extent, pos + extent)
        }
        FigureType::Cone | FigureType::Paraboloid | FigureType::Pyramid => {
            let (r, h) = (fig.radius, fig.height);
            Aabb::new(pos - Vec3::new(r, 0.0, r), pos + Vec3::new(r, h, r))
        }
        FigureType::Torus => {
            let extent = fig.radius + fig.radius2;
            Aabb::new(
                pos - Vec3::new(extent, fig.radius2, extent),
                pos + Vec3::new(extent, fig.radius2, extent),
            )
        }
        FigureType::Disc => {
            Aabb::new(pos - Vec3::splat(fig.radius), pos + Vec3::splat(fig.radius)).pad()
        }
        FigureType::Triangle => Aabb::from_point(Vec3::from(fig.v0))
            .expand(Vec3::from(fig.v1))
            .expand(Vec3::from(fig.v2))
            .pad(),
        FigureType::Mandelbulb | FigureType::Julia => {
            let r = Vec3::splat(fig.radius * 1.5);
            Aabb::new(pos - r, pos + r)
        }
        FigureType::Ellipsoid => {
            // radius = x-radius, radius2 = z-radius, height = y-radius
            let extent = Vec3::new(
                fig.radius,
                fig.height.max(fig.radius),
                fig.radius2.max(fig.radius),
            );
            Aabb::new(pos - extent, pos + extent)
        }
        FigureType::Hyperboloid => {
            let h = fig.height * 0.5;
            let extent = Vec3::new(fig.radius + h, h, fig.radius + h);
            Aabb::new(pos - extent, pos + extent)
        }
        FigureType::Mebius => {
            let extent = Vec3::splat(fig.radius * 1.5);
            Aabb::new(pos - extent, pos + extent)
        }
        FigureType::Tetrahedron => {
            let extent = Vec3::splat(fig.radius);
            Aabb::new(pos - extent, pos + extent)
        }
        // Infinite primitives — given a large finite box so the BVH builder
        // can still include them; the shader handles their true intersection.
        FigureType::Plane | FigureType::Skybox => {
            let big = Vec3::splat(1e6);
            Aabb::new(-big, big)
        }
    }
}
