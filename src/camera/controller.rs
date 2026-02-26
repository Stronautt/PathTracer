use glam::Vec3;

use super::camera::Camera;

const DEFAULT_SENSITIVITY: f32 = 0.15;

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
            move_speed: 5.0,
            look_sensitivity,
            sprint_multiplier: 3.0,
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
            return DEFAULT_SENSITIVITY;
        };
        match val.parse::<f32>() {
            Ok(sens) if sens > 0.0 && sens.is_finite() => {
                log::info!("PATHTRACER_MOUSE_SENS={sens}");
                sens
            }
            _ => {
                log::warn!("PATHTRACER_MOUSE_SENS={val:?} invalid, using default");
                DEFAULT_SENSITIVITY
            }
        }
    }

    /// Returns true if the camera moved (signals accumulation reset).
    pub fn update(&mut self, camera: &mut Camera, dt: f32) -> bool {
        if self.speed_up {
            self.move_speed = (self.move_speed + 5.0 * dt).min(50.0);
        }
        if self.speed_down {
            self.move_speed = (self.move_speed - 5.0 * dt).max(0.5);
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
        let is_absolute = x.abs() > 5000.0 || y.abs() > 5000.0;

        let (dx, dy) = if !is_absolute {
            self.last_raw_pos = None;
            (x as f32, y as f32)
        } else {
            let delta = self.last_raw_pos.and_then(|(lx, ly)| {
                let dx = (x - lx) as f32;
                let dy = (y - ly) as f32;
                if (dx != 0.0 || dy != 0.0) && dx.abs() < 500.0 && dy.abs() < 500.0 {
                    const RAW_SCALE: f32 = 0.05;
                    Some((dx * RAW_SCALE, dy * RAW_SCALE))
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
            camera.yaw - dx * self.look_sensitivity,
            camera.pitch,
            (camera.pitch - dy * self.look_sensitivity).clamp(-89.0, 89.0),
        );
        camera.yaw -= dx * self.look_sensitivity;
        camera.pitch = (camera.pitch - dy * self.look_sensitivity).clamp(-89.0, 89.0);
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
