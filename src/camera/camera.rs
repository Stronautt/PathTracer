// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

use crate::constants::{
    DEFAULT_CAMERA_POSITION, DEFAULT_EXPOSURE, DEFAULT_FIREFLY_CLAMP, DEFAULT_FOV,
    DEFAULT_FRACTAL_MARCH_STEPS, DEFAULT_MAX_BOUNCES, DEFAULT_SKYBOX_BRIGHTNESS,
    DEFAULT_SKYBOX_COLOR, DEFAULT_TONE_MAPPER,
};
use crate::scene::scene::CameraConfig;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees
    pub fov: f32,   // degrees
    pub exposure: f32,
    pub max_bounces: u32,
    pub tone_mapper: u32,
    pub fractal_march_steps: u32,
    pub firefly_clamp: f32,
    pub skybox_color: [f32; 3],
    pub skybox_brightness: f32,
}

impl Camera {
    pub fn new(position: Vec3, rotation: [f32; 3], fov: f32, exposure: f32) -> Self {
        Self {
            position,
            yaw: rotation[1],
            pitch: rotation[0],
            fov,
            exposure,
            max_bounces: DEFAULT_MAX_BOUNCES,
            tone_mapper: DEFAULT_TONE_MAPPER,
            fractal_march_steps: DEFAULT_FRACTAL_MARCH_STEPS,
            firefly_clamp: DEFAULT_FIREFLY_CLAMP,
            skybox_color: DEFAULT_SKYBOX_COLOR,
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        }
    }

    /// Construct a camera fully from a scene's camera config (position, orientation, and all
    /// render settings). Prefer this over `new()` followed by manual field assignments.
    pub fn from_config(cfg: &CameraConfig) -> Self {
        let mut cam = Self::new(cfg.position.into(), cfg.rotation, cfg.fov, cfg.exposure);
        cam.apply_render_settings(cfg);
        cam
    }

    /// Serialize the camera back into a `CameraConfig` for scene saving.
    pub fn to_config(&self) -> CameraConfig {
        CameraConfig {
            position: self.position.into(),
            rotation: [self.pitch, self.yaw, 0.0],
            fov: self.fov,
            exposure: self.exposure,
            max_bounces: self.max_bounces,
            firefly_clamp: self.firefly_clamp,
            skybox_color: self.skybox_color,
            skybox_brightness: self.skybox_brightness,
            tone_mapper: self.tone_mapper,
            fractal_march_steps: self.fractal_march_steps,
        }
    }

    /// Copy the render settings (everything except position/orientation/fov/exposure) from a
    /// `CameraConfig` into this camera.
    pub fn apply_render_settings(&mut self, cfg: &CameraConfig) {
        self.max_bounces = cfg.max_bounces;
        self.firefly_clamp = cfg.firefly_clamp;
        self.skybox_color = cfg.skybox_color;
        self.skybox_brightness = cfg.skybox_brightness;
        self.tone_mapper = cfg.tone_mapper;
        self.fractal_march_steps = cfg.fractal_march_steps;
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
            max_bounces: self.max_bounces,
            tone_mapper: self.tone_mapper,
            fractal_march_steps: self.fractal_march_steps,
            firefly_clamp: self.firefly_clamp,
            skybox_brightness: self.skybox_brightness,
            skybox_color: self.skybox_color,
            _pad2: 0.0,
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
            max_bounces: DEFAULT_MAX_BOUNCES,
            tone_mapper: DEFAULT_TONE_MAPPER,
            fractal_march_steps: DEFAULT_FRACTAL_MARCH_STEPS,
            firefly_clamp: DEFAULT_FIREFLY_CLAMP,
            skybox_color: DEFAULT_SKYBOX_COLOR,
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
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
    pub max_bounces: u32,
    pub tone_mapper: u32,
    pub fractal_march_steps: u32,
    pub firefly_clamp: f32,
    pub skybox_brightness: f32,
    pub skybox_color: [f32; 3],
    pub _pad2: f32,
}
