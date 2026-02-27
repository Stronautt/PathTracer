// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod object_editor;
pub mod toolbar;

use egui::{Color32, Context, RichText};

use std::path::PathBuf;

use crate::constants::{
    DEFAULT_COMIC_LEVELS, DEFAULT_FIREFLY_CLAMP, DEFAULT_FRACTAL_MARCH_STEPS, DEFAULT_MAX_BOUNCES,
    DEFAULT_OIL_RADIUS, DEFAULT_SKYBOX_BRIGHTNESS, DEFAULT_SKYBOX_COLOR, DEFAULT_TONE_MAPPER,
};
use crate::render::post_process::PostEffect;
use crate::scene::shape::{Shape, ShapeType};

/// Extension trait that sets a pointing-hand cursor on hover for interactive widgets.
pub(crate) trait Pointer {
    fn pointer(self) -> Self;
}

impl Pointer for egui::Response {
    fn pointer(self) -> Self {
        self.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

#[derive(Default)]
pub struct UiActions {
    pub screenshot_path: Option<String>,
    pub save_requested: bool,
    pub paused: bool,
    pub exposure_changed: Option<f32>,
    pub max_bounces_changed: Option<u32>,
    pub effects_changed: Option<Vec<PostEffect>>,
    pub shape_to_add: Option<ShapeType>,
    pub selected_shape: Option<usize>,
    pub scene_dirty: bool,
    pub textures_dirty: bool,
    pub shape_to_delete: Option<usize>,
    pub import_scene_path: Option<PathBuf>,
    pub import_model_path: Option<PathBuf>,
    /// Scale ratio to apply to the selected model group (new_scale / old_scale).
    pub model_scale_ratio: Option<f32>,
    pub render_settings_changed: bool,
    pub post_effect_params_changed: bool,
    /// Signal the app to open a file dialog on a background thread.
    pub open_scene_dialog: bool,
    pub open_import_scene_dialog: bool,
    pub open_import_model_dialog: bool,
    /// Open a bundled example scene by its resolved path.
    pub open_example_scene: Option<PathBuf>,
}

pub struct UiState {
    pub paused: bool,
    pub active_effects: Vec<PostEffect>,
    pub exposure: f32,
    pub max_bounces: u32,
    pub selected_shape: Option<usize>,
    pub fps: f32,
    pub sample_count: u32,
    pub render_elapsed_secs: f32,
    pub save_dialog_open: bool,
    pub save_filename: String,
    pub confirm_delete_shape: Option<usize>,
    pub confirm_overwrite_save: bool,
    pub screenshot_dialog_open: bool,
    pub screenshot_filename: String,
    pub firefly_clamp: f32,
    pub skybox_color: [f32; 3],
    pub skybox_brightness: f32,
    pub tone_mapper: u32,
    pub fractal_march_steps: u32,
    pub oil_radius: u32,
    pub comic_levels: u32,
    /// Current scale for the selected model group (for the scale slider).
    pub model_scale: f32,
    /// Cached list of example scene stem names.
    pub example_scenes: Vec<String>,
    pub shortcuts_dialog_open: bool,
    pub about_dialog_open: bool,
}

impl UiState {
    /// Mirror camera render settings into UI state so sliders stay in sync after a scene load.
    pub fn sync_from_camera(&mut self, camera: &crate::camera::camera::Camera) {
        self.exposure = camera.exposure;
        self.max_bounces = camera.max_bounces;
        self.firefly_clamp = camera.firefly_clamp;
        self.skybox_color = camera.skybox_color;
        self.skybox_brightness = camera.skybox_brightness;
        self.tone_mapper = camera.tone_mapper;
        self.fractal_march_steps = camera.fractal_march_steps;
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: false,
            active_effects: Vec::new(),
            exposure: 1.0,
            max_bounces: DEFAULT_MAX_BOUNCES,
            selected_shape: None,
            fps: 0.0,
            sample_count: 0,
            render_elapsed_secs: 0.0,
            save_dialog_open: false,
            save_filename: "scene_saved.yaml".to_string(),
            confirm_delete_shape: None,
            confirm_overwrite_save: false,
            screenshot_dialog_open: false,
            screenshot_filename: String::new(),
            firefly_clamp: DEFAULT_FIREFLY_CLAMP,
            skybox_color: DEFAULT_SKYBOX_COLOR,
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
            tone_mapper: DEFAULT_TONE_MAPPER,
            fractal_march_steps: DEFAULT_FRACTAL_MARCH_STEPS,
            oil_radius: DEFAULT_OIL_RADIUS,
            comic_levels: DEFAULT_COMIC_LEVELS,
            model_scale: 1.0,
            example_scenes: Vec::new(),
            shortcuts_dialog_open: false,
            about_dialog_open: false,
        }
    }
}

pub fn draw_ui(ctx: &Context, state: &mut UiState, shapes: &mut [Shape]) -> UiActions {
    let mut actions = UiActions::default();

    toolbar::draw_toolbar(ctx, state, shapes, &mut actions);

    // --- Welcome screen (shown when the scene is empty) ---
    if shapes.is_empty() {
        egui::Area::new(egui::Id::new("welcome_screen"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(340.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("Welcome to PathTracer");
                    });
                    ui.add_space(8.0);
                    ui.label("Get started:");
                    ui.add_space(4.0);
                    egui::Grid::new("welcome_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("Scene > Open");
                            ui.label("Load a scene from disk");
                            ui.end_row();
                            ui.strong("Scene > Examples");
                            ui.label("Browse bundled example scenes");
                            ui.end_row();
                            ui.strong("Scene > Add Shape");
                            ui.label("Create a new primitive");
                            ui.end_row();
                        });
                });
            });
    }

    if let Some(idx) = state.selected_shape
        && idx < shapes.len()
    {
        object_editor::draw_object_editor(ctx, state, &mut shapes[idx], idx, &mut actions);

        // Propagate material/texture changes to all group members (same name).
        if actions.scene_dirty
            && shapes[idx].shape_type == ShapeType::Triangle
            && let Some(name) = shapes[idx].name.clone()
            && !name.is_empty()
        {
            let mat = shapes[idx].material.clone();
            let neg = shapes[idx].negative;
            let tex = shapes[idx].texture.clone();
            let tex_scale = shapes[idx].texture_scale;
            for (i, s) in shapes.iter_mut().enumerate() {
                if i != idx
                    && s.shape_type == ShapeType::Triangle
                    && s.name.as_deref() == Some(&name)
                {
                    s.material = mat.clone();
                    s.negative = neg;
                    s.texture = tex.clone();
                    s.texture_scale = tex_scale;
                }
            }
        }

        // Apply scale to the entire model group.
        if let Some(ratio) = actions.model_scale_ratio
            && shapes[idx].shape_type == ShapeType::Triangle
        {
            let group_name = shapes[idx].name.clone();
            scale_model_group(shapes, &group_name, ratio);
            actions.scene_dirty = true;
        }
    }

    // --- Save dialog modal ---
    if state.save_dialog_open {
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new("Save Scene")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("File name:");
                let response = ui.text_edit_singleline(&mut state.save_filename);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirmed = true;
                }
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Save").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(60, 120, 200)),
                            )
                            .pointer()
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button("Cancel").pointer().clicked() {
                            cancelled = true;
                        }
                    });
                });
            });
        if confirmed && !state.save_filename.trim().is_empty() {
            if std::path::Path::new(state.save_filename.trim()).exists() {
                state.save_dialog_open = false;
                state.confirm_overwrite_save = true;
            } else {
                actions.save_requested = true;
                state.save_dialog_open = false;
            }
        } else if cancelled {
            state.save_dialog_open = false;
        }
    }

    // --- Overwrite confirmation modal ---
    if state.confirm_overwrite_save {
        let mut resolved = false;
        egui::Window::new("Overwrite File")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "\"{}\" already exists. Overwrite?",
                    state.save_filename
                ));
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Overwrite").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(200, 60, 60)),
                            )
                            .pointer()
                            .clicked()
                        {
                            actions.save_requested = true;
                            resolved = true;
                        }
                        if ui.button("Cancel").pointer().clicked() {
                            resolved = true;
                        }
                    });
                });
            });
        if resolved {
            state.confirm_overwrite_save = false;
        }
    }

    // --- Delete confirmation modal ---
    if let Some(idx) = state.confirm_delete_shape {
        let label = if idx < shapes.len() {
            shape_label(&shapes[idx], idx)
        } else {
            format!("Shape #{idx}")
        };
        let mut resolved = false;
        egui::Window::new("Delete Shape")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Remove {label} from the scene?"));
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Delete").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(200, 60, 60)),
                            )
                            .pointer()
                            .clicked()
                        {
                            actions.shape_to_delete = Some(idx);
                            resolved = true;
                        }
                        if ui.button("Cancel").pointer().clicked() {
                            resolved = true;
                        }
                    });
                });
            });
        if resolved {
            state.confirm_delete_shape = None;
        }
    }

    // --- Screenshot dialog modal ---
    if state.screenshot_dialog_open {
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new("Screenshot")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("File name:");
                let response = ui.text_edit_singleline(&mut state.screenshot_filename);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirmed = true;
                }
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Save").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(60, 120, 200)),
                            )
                            .pointer()
                            .clicked()
                        {
                            confirmed = true;
                        }
                        if ui.button("Cancel").pointer().clicked() {
                            cancelled = true;
                        }
                    });
                });
            });
        if confirmed && !state.screenshot_filename.trim().is_empty() {
            actions.screenshot_path = Some(state.screenshot_filename.clone());
            state.screenshot_dialog_open = false;
        } else if cancelled {
            state.screenshot_dialog_open = false;
        }
    }

    // --- Shortcuts dialog ---
    if state.shortcuts_dialog_open {
        let mut open = true;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        let shortcuts = [
                            ("W / A / S / D", "Camera movement"),
                            ("Space", "Move up"),
                            ("Ctrl", "Move down"),
                            ("Shift", "Sprint"),
                            ("M", "Toggle mouse look"),
                            ("Right Mouse", "Capture mouse"),
                            ("Left Mouse", "Select / drag shape"),
                            ("Numpad + / -", "Camera speed"),
                            ("Escape", "Release mouse / Exit"),
                        ];
                        for (key, desc) in shortcuts {
                            ui.strong(key);
                            ui.label(desc);
                            ui.end_row();
                        }
                    });
            });
        if !open {
            state.shortcuts_dialog_open = false;
        }
    }

    // --- About dialog ---
    if state.about_dialog_open {
        let mut open = true;
        egui::Window::new("About")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("PathTracer");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(6.0);
                ui.label("GPU-accelerated 3D PBR path tracer");
                ui.add_space(6.0);
                ui.label("Author: Pavlo Hrytsenko");
                ui.label("License: GPL-3.0-or-later");
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Inspired by the RT project from 42 school (Unit Factory)")
                        .italics(),
                );
            });
        if !open {
            state.about_dialog_open = false;
        }
    }

    actions
}

/// Scale all triangles in a model group by `ratio` relative to the group's centroid.
fn scale_model_group(shapes: &mut [Shape], group_name: &Option<String>, ratio: f32) {
    use glam::Vec3;

    let name = match group_name {
        Some(n) if !n.is_empty() => n.as_str(),
        _ => return,
    };

    // Collect indices of group members.
    let indices: Vec<usize> = shapes
        .iter()
        .enumerate()
        .filter(|(_, s)| s.shape_type == ShapeType::Triangle && s.name.as_deref() == Some(name))
        .map(|(i, _)| i)
        .collect();

    if indices.is_empty() {
        return;
    }

    // Compute centroid of the entire group.
    let mut sum = Vec3::ZERO;
    let mut count = 0u32;
    for &i in &indices {
        sum += Vec3::from(shapes[i].v0);
        sum += Vec3::from(shapes[i].v1);
        sum += Vec3::from(shapes[i].v2);
        count += 3;
    }
    let center = sum / count as f32;

    // Scale each vertex relative to the centroid.
    for &i in &indices {
        let s = &mut shapes[i];
        let v0 = center + (Vec3::from(s.v0) - center) * ratio;
        let v1 = center + (Vec3::from(s.v1) - center) * ratio;
        let v2 = center + (Vec3::from(s.v2) - center) * ratio;
        s.v0 = v0.into();
        s.v1 = v1.into();
        s.v2 = v2.into();
    }
}

pub fn shape_label(shape: &Shape, idx: usize) -> String {
    match &shape.name {
        Some(name) if !name.is_empty() => name.clone(),
        _ => format!("{} #{}", shape.shape_type.label(), idx),
    }
}
