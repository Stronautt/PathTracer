// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use glam::Vec3;

use super::camera::Camera;
use crate::constants::{
    CAMERA_DEFAULT_MOVE_SPEED, CAMERA_DEFAULT_SENSITIVITY, CAMERA_PITCH_CLAMP,
    CAMERA_RAW_ABSOLUTE_THRESHOLD, CAMERA_RAW_JUMP_THRESHOLD, CAMERA_RAW_SCALE, CAMERA_SPEED_MAX,
    CAMERA_SPEED_MIN, CAMERA_SPEED_STEP, CAMERA_SPRINT_MULTIPLIER,
};

/// FPS-style camera controller (WASD + mouse look).
pub struct CameraController {
    pub move_speed: f32,
    pub look_sensitivity: f32,
    pub sprint_multiplier: f32,
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub mouse_captured: bool,
    pub speed_up: bool,
    pub speed_down: bool,
    pub mouse_look_key: bool,
    mouse_delta: (f32, f32),
    last_cursor_pos: Option<(f32, f32)>,
    // Last raw device position (for VM absolute-coordinate detection)
    last_raw_pos: Option<(f64, f64)>,
}

impl CameraController {
    pub fn new() -> Self {
        let look_sensitivity = Self::resolve_sensitivity();

        Self {
            move_speed: CAMERA_DEFAULT_MOVE_SPEED,
            look_sensitivity,
            sprint_multiplier: CAMERA_SPRINT_MULTIPLIER,
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            sprint: false,
            mouse_captured: false,
            speed_up: false,
            speed_down: false,
            mouse_look_key: false,
            mouse_delta: (0.0, 0.0),
            last_cursor_pos: None,
            last_raw_pos: None,
        }
    }

    fn resolve_sensitivity() -> f32 {
        let Ok(val) = std::env::var("PATHTRACER_MOUSE_SENS") else {
            return CAMERA_DEFAULT_SENSITIVITY;
        };
        match val.parse::<f32>() {
            Ok(sens) if sens > 0.0 && sens.is_finite() => {
                log::info!("PATHTRACER_MOUSE_SENS={sens}");
                sens
            }
            _ => {
                log::warn!("PATHTRACER_MOUSE_SENS={val:?} invalid, using default");
                CAMERA_DEFAULT_SENSITIVITY
            }
        }
    }

    /// Returns true if the camera moved (signals accumulation reset).
    pub fn update(&mut self, camera: &mut Camera, dt: f32) -> bool {
        if self.speed_up {
            self.move_speed = (self.move_speed + CAMERA_SPEED_STEP * dt).min(CAMERA_SPEED_MAX);
        }
        if self.speed_down {
            self.move_speed = (self.move_speed - CAMERA_SPEED_STEP * dt).max(CAMERA_SPEED_MIN);
        }

        let sprint_factor = if self.sprint {
            self.sprint_multiplier
        } else {
            1.0
        };
        let speed = self.move_speed * sprint_factor * dt;
        let (cam_right, _cam_up, cam_forward) = camera.basis_vectors();

        let mut delta = Vec3::ZERO;
        if self.forward {
            delta += cam_forward;
        }
        if self.backward {
            delta -= cam_forward;
        }
        if self.right {
            delta += cam_right;
        }
        if self.left {
            delta -= cam_right;
        }
        if self.up {
            delta += Vec3::Y;
        }
        if self.down {
            delta -= Vec3::Y;
        }

        if delta != Vec3::ZERO {
            camera.position += delta.normalize() * speed;
            true
        } else {
            false
        }
    }

    pub fn handle_cursor_moved(&mut self, x: f32, y: f32) {
        self.last_cursor_pos = Some((x, y));
    }

    /// Accumulate mouse movement from `DeviceEvent::MouseMotion`.
    ///
    /// Some VMs report absolute tablet coordinates (values in the thousands)
    /// instead of relative deltas. A threshold of 5000 separates the two cases
    /// and converts absolute positions to relative deltas via frame differencing.
    pub fn accumulate_raw_delta(&mut self, x: f64, y: f64) {
        let is_absolute =
            x.abs() > CAMERA_RAW_ABSOLUTE_THRESHOLD || y.abs() > CAMERA_RAW_ABSOLUTE_THRESHOLD;

        let (dx, dy) = if !is_absolute {
            self.last_raw_pos = None;
            (x as f32, y as f32)
        } else {
            let delta = self.last_raw_pos.and_then(|(lx, ly)| {
                let dx = (x - lx) as f32;
                let dy = (y - ly) as f32;
                if (dx != 0.0 || dy != 0.0)
                    && dx.abs() < CAMERA_RAW_JUMP_THRESHOLD
                    && dy.abs() < CAMERA_RAW_JUMP_THRESHOLD
                {
                    Some((dx * CAMERA_RAW_SCALE, dy * CAMERA_RAW_SCALE))
                } else {
                    None
                }
            });
            self.last_raw_pos = Some((x, y));
            match delta {
                Some(d) => d,
                None => return,
            }
        };

        if self.mouse_captured || self.mouse_look_key {
            self.mouse_delta.0 += dx;
            self.mouse_delta.1 += dy;
        }
    }

    /// Apply accumulated mouse delta to camera rotation (called once per frame).
    /// Returns true if camera rotated (signals accumulation reset).
    pub fn apply_mouse_look(&mut self, camera: &mut Camera) -> bool {
        let (dx, dy) = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        log::debug!(
            "[mouse] frame delta: ({dx:.2}, {dy:.2}), yaw: {:.2} -> {:.2}, pitch: {:.2} -> {:.2}",
            camera.yaw,
            camera.yaw + dx * self.look_sensitivity,
            camera.pitch,
            (camera.pitch + dy * self.look_sensitivity)
                .clamp(-CAMERA_PITCH_CLAMP, CAMERA_PITCH_CLAMP),
        );
        camera.yaw += dx * self.look_sensitivity;
        camera.pitch = (camera.pitch + dy * self.look_sensitivity)
            .clamp(-CAMERA_PITCH_CLAMP, CAMERA_PITCH_CLAMP);
        true
    }

    pub fn last_cursor_pos(&self) -> Option<(f32, f32)> {
        self.last_cursor_pos
    }

    /// Discard buffered mouse delta (call when toggling mouse capture to avoid a jump).
    pub fn clear_mouse_delta(&mut self) {
        self.mouse_delta = (0.0, 0.0);
        self.last_raw_pos = None;
    }

    /// Reset all movement flags (call on focus loss to prevent runaway movement).
    pub fn clear_movement(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.down = false;
        self.sprint = false;
        self.speed_up = false;
        self.speed_down = false;
    }
}
