// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

use crate::constants::{DEFAULT_CAMERA_POSITION, DEFAULT_EXPOSURE, DEFAULT_FOV};

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees
    pub fov: f32,   // degrees
    pub exposure: f32,
}

impl Camera {
    pub fn new(position: Vec3, rotation: [f32; 3], fov: f32, exposure: f32) -> Self {
        Self {
            position,
            yaw: rotation[1],
            pitch: rotation[0],
            fov,
            exposure,
        }
    }

    pub fn orientation(&self) -> Quat {
        Quat::from_euler(
            glam::EulerRot::YXZ,
            self.yaw.to_radians(),
            self.pitch.to_radians(),
            0.0,
        )
    }

    pub fn basis_vectors(&self) -> (Vec3, Vec3, Vec3) {
        let rot = self.orientation();
        let forward = rot * Vec3::Z;
        let right = rot * Vec3::X;
        let up = rot * Vec3::Y;
        (right, up, forward)
    }

    pub fn to_gpu(
        &self,
        width: u32,
        height: u32,
        frame_index: u32,
        sample_count: u32,
    ) -> GpuCamera {
        let (right, up, forward) = self.basis_vectors();
        let aspect = width as f32 / height as f32;
        let focal_length = 1.0 / (self.fov.to_radians() * 0.5).tan();

        GpuCamera {
            position: self.position.into(),
            focal_length,
            right: right.into(),
            aspect,
            up: up.into(),
            exposure: self.exposure,
            forward: forward.into(),
            frame_index,
            width,
            height,
            sample_count,
            _pad: 0,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::from(DEFAULT_CAMERA_POSITION),
            yaw: 0.0,
            pitch: 0.0,
            fov: DEFAULT_FOV,
            exposure: DEFAULT_EXPOSURE,
        }
    }
}

/// Must match the WGSL `Camera` struct layout exactly.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuCamera {
    pub position: [f32; 3],
    pub focal_length: f32,
    pub right: [f32; 3],
    pub aspect: f32,
    pub up: [f32; 3],
    pub exposure: f32,
    pub forward: [f32; 3],
    pub frame_index: u32,
    pub width: u32,
    pub height: u32,
    pub sample_count: u32,
    pub _pad: u32,
}
