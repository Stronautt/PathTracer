use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::camera::CameraController;

/// Returns true if the event was consumed.
pub fn handle_window_event(event: &WindowEvent, controller: &mut CameraController) -> bool {
    match event {
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                },
            ..
        } => {
            let pressed = *state == ElementState::Pressed;
            match key {
                KeyCode::KeyW => controller.forward = pressed,
                KeyCode::KeyS => controller.backward = pressed,
                KeyCode::KeyA => controller.left = pressed,
                KeyCode::KeyD => controller.right = pressed,
                KeyCode::Space => controller.up = pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => controller.sprint = pressed,
                KeyCode::ControlLeft | KeyCode::ControlRight => controller.down = pressed,
                KeyCode::NumpadAdd => controller.speed_up = pressed,
                KeyCode::NumpadSubtract => controller.speed_down = pressed,
                KeyCode::KeyM => {
                    if pressed {
                        controller.mouse_look_key = !controller.mouse_look_key;
                    }
                }
                _ => return false,
            }
            true
        }
        WindowEvent::MouseInput {
            button: MouseButton::Right,
            state,
            ..
        } => {
            controller.mouse_captured = *state == ElementState::Pressed;
            true
        }
        _ => false,
    }
}
