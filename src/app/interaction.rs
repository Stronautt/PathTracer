// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::constants::DRAG_THRESHOLD_PX;
use crate::input::handler;
use crate::scene::shape::ShapeType;

use super::state::{AppState, FileDialogResult};

/// Compute the effective center of a shape for drag purposes.
/// For triangles, uses the centroid of v0/v1/v2; for others, uses `position`.
pub fn shape_centroid(shape: &crate::scene::shape::Shape) -> glam::Vec3 {
    if shape.shape_type == ShapeType::Triangle {
        let v0 = glam::Vec3::from(shape.v0);
        let v1 = glam::Vec3::from(shape.v1);
        let v2 = glam::Vec3::from(shape.v2);
        (v0 + v1 + v2) / 3.0
    } else {
        glam::Vec3::from(shape.position)
    }
}

/// Translate a shape to `new_pos`.
///
/// For named triangles all triangles sharing the same name (i.e. the same OBJ
/// mesh group) are translated together so the model stays coherent.
/// Unnamed lone triangles and all other shape types are moved individually.
pub fn move_shape_or_group(
    shapes: &mut [crate::scene::shape::Shape],
    idx: usize,
    new_pos: glam::Vec3,
) {
    let shape = &shapes[idx];
    if shape.shape_type == ShapeType::Triangle {
        let old_centroid = shape_centroid(shape);
        let delta = new_pos - old_centroid;
        // Only group-move if the triangle has a non-empty name.
        let group_name = shape.name.as_deref().filter(|n| !n.is_empty());
        if let Some(name) = group_name {
            let name = name.to_string();
            for s in shapes.iter_mut() {
                if s.shape_type == ShapeType::Triangle && s.name.as_deref() == Some(&name) {
                    let v0 = glam::Vec3::from(s.v0) + delta;
                    let v1 = glam::Vec3::from(s.v1) + delta;
                    let v2 = glam::Vec3::from(s.v2) + delta;
                    s.v0 = v0.into();
                    s.v1 = v1.into();
                    s.v2 = v2.into();
                }
            }
        } else {
            // Lone unnamed triangle — move just this one.
            let v0 = glam::Vec3::from(shapes[idx].v0) + delta;
            let v1 = glam::Vec3::from(shapes[idx].v1) + delta;
            let v2 = glam::Vec3::from(shapes[idx].v2) + delta;
            shapes[idx].v0 = v0.into();
            shapes[idx].v1 = v1.into();
            shapes[idx].v2 = v2.into();
        }
    } else {
        shapes[idx].position = new_pos.into();
    }
}

pub fn handle_window_event(state: &mut AppState, event_loop: &ActiveEventLoop, event: WindowEvent) {
    let is_keyboard = matches!(&event, WindowEvent::KeyboardInput { .. });
    let egui_wants_kb = state.egui_ctx.wants_keyboard_input();

    if is_keyboard {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = &event
            && key_event.physical_key == PhysicalKey::Code(KeyCode::Escape)
        {
            if state.controller.mouse_look_key {
                state.controller.mouse_look_key = false;
                state.set_cursor_grabbed(false);
                state.controller.clear_mouse_delta();
            } else if state.controller.mouse_captured {
                state.controller.mouse_captured = false;
            } else {
                event_loop.exit();
            }
            return;
        }
        if !egui_wants_kb {
            if let WindowEvent::KeyboardInput {
                event: ref key_event,
                ..
            } = event
                && key_event.physical_key == PhysicalKey::Code(KeyCode::F12)
                && key_event.state == ElementState::Pressed
            {
                let tx = state.file_dialog_tx.clone();
                let default_name = crate::io::screenshot::default_screenshot_path()
                    .to_string_lossy()
                    .to_string();
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG image", &["png"])
                        .set_file_name(&default_name)
                        .save_file()
                    {
                        let _ = tx.send(FileDialogResult::Screenshot(path));
                    }
                });
            }

            let was_mouse_look = state.controller.mouse_look_key;
            handler::handle_window_event(&event, &mut state.controller);
            if state.controller.mouse_look_key != was_mouse_look {
                state.set_cursor_grabbed(state.controller.mouse_look_key);
                state.controller.clear_mouse_delta();
            }
        }
    }

    if let WindowEvent::CursorMoved { position, .. } = &event {
        state
            .controller
            .handle_cursor_moved(position.x as f32, position.y as f32);
    }

    // In mouse look mode, don't forward events to egui so the UI doesn't
    // react to mouse movement / clicks and doesn't override cursor visibility.
    let in_mouse_look = state.controller.mouse_look_key || state.controller.mouse_captured;
    if !in_mouse_look {
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        if egui_response.consumed {
            return;
        }
    }

    match &event {
        WindowEvent::CloseRequested => {
            event_loop.exit();
        }
        WindowEvent::Resized(size) => {
            state.handle_resize(*size);
        }
        WindowEvent::RedrawRequested => {
            state.update_and_render();
            return;
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        } if !state.controller.mouse_captured && !state.controller.mouse_look_key => {
            if let Some((cx, cy)) = state.controller.last_cursor_pos() {
                let (origin, dir) = crate::picking::picking_ray(
                    &state.camera,
                    cx,
                    cy,
                    state.gpu.width(),
                    state.gpu.height(),
                );
                if let Some((idx, t, hit_point)) = crate::picking::pick(
                    origin,
                    dir,
                    &state.bvh,
                    &state.shapes,
                    &state.infinite_indices,
                ) {
                    let shape_pos = shape_centroid(&state.shapes[idx]);
                    state.drag_shape = Some(idx);
                    state.drag_depth = t;
                    state.drag_offset = hit_point - shape_pos;
                    state.drag_moved = false;
                    state.drag_start_pos = (cx, cy);
                } else {
                    state.ui_state.selected_shape = None;
                    state.drag_shape = None;
                }
            }
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Released,
            ..
        } => {
            if let Some(idx) = state.drag_shape.take() {
                if state.drag_moved {
                    // Drag finished — do full BVH rebuild now.
                    state.rebuild_scene_buffers();
                } else {
                    // Click without drag — select the shape.
                    state.ui_state.selected_shape = Some(idx);
                    state.ui_state.model_scale = 1.0;
                }
            }
        }
        WindowEvent::CursorMoved { position, .. } if state.drag_shape.is_some() => {
            let px = position.x as f32;
            let py = position.y as f32;
            let (sx, sy) = state.drag_start_pos;
            let dist_sq = (px - sx).powi(2) + (py - sy).powi(2);

            // Threshold comparison in squared space avoids a sqrt.
            if dist_sq >= DRAG_THRESHOLD_PX * DRAG_THRESHOLD_PX {
                let idx = state.drag_shape.unwrap();
                state.drag_moved = true;
                let (origin, dir) = crate::picking::picking_ray(
                    &state.camera,
                    px,
                    py,
                    state.gpu.width(),
                    state.gpu.height(),
                );
                let new_pos = origin + dir * state.drag_depth - state.drag_offset;
                move_shape_or_group(&mut state.shapes, idx, new_pos);
                state.rebuild_scene_buffers_in_place();
                state.accumulator.reset();
            }
        }
        // Focus loss: release cursor and clear all input state so camera
        // doesn't keep moving when the user alt-tabs away.
        WindowEvent::Focused(false) => {
            state.controller.mouse_look_key = false;
            state.controller.mouse_captured = false;
            state.controller.clear_movement();
            state.controller.clear_mouse_delta();
            state.set_cursor_grabbed(false);
        }
        _ => {}
    }

    if !is_keyboard {
        let was_captured = state.controller.mouse_captured;
        handler::handle_window_event(&event, &mut state.controller);
        if state.controller.mouse_captured != was_captured {
            state.controller.clear_mouse_delta();
        }
    }
}
