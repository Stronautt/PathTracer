// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

use egui::{Color32, Context, Ui};

use super::{Pointer, UiActions, UiState, shape_label};
use crate::scene::material::Material;
use crate::scene::shape::{Shape, ShapeType};

pub fn draw_object_editor(
    ctx: &Context,
    state: &mut UiState,
    shape: &mut Shape,
    shape_idx: usize,
    actions: &mut UiActions,
) {
    egui::SidePanel::right("object_editor")
        .min_width(200.0)
        .max_width(240.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;

                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.strong(shape_label(shape, shape_idx));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").pointer().clicked() {
                                state.selected_shape = None;
                            }
                            if ui.small_button("🗑").pointer().clicked() {
                                state.confirm_delete_shape = Some(shape_idx);
                            }
                        });
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let name = shape.name.get_or_insert_default();
                        ui.text_edit_singleline(name);
                    });

                    if shape.negative {
                        ui.colored_label(Color32::YELLOW, "⚠ Negative (CSG subtraction)");
                    }

                    let is_triangle = shape.shape_type == ShapeType::Triangle;

                    if is_triangle {
                        let prev = state.model_scale;
                        if ui
                            .add(
                                egui::Slider::new(&mut state.model_scale, 0.01..=10.0)
                                    .text("Scale")
                                    .logarithmic(true),
                            )
                            .pointer()
                            .changed()
                        {
                            actions.model_scale_ratio = Some(state.model_scale / prev);
                        }
                    }

                    if !is_triangle {
                        ui.label("Position");
                        changed |= drag_vec3(ui, &mut shape.position, 0.1, None);
                    }

                    let is_fractal =
                        matches!(shape.shape_type, ShapeType::Mandelbulb | ShapeType::Julia);

                    if !is_triangle {
                        if shape.shape_type == ShapeType::Julia {
                            ui.label("Julia C");
                            changed |= drag_vec3(ui, &mut shape.rotation, 0.01, Some(-2.0..=2.0));
                            changed |= ui
                                .add(egui::Slider::new(&mut shape.radius2, -2.0..=2.0).text("C.w"))
                                .pointer()
                                .changed();
                        } else if !is_fractal {
                            ui.label("Rotation");
                            changed |= drag_vec3_deg(ui, &mut shape.rotation, 1.0);
                        }

                        let has_normal = matches!(
                            shape.shape_type,
                            ShapeType::Plane
                                | ShapeType::Disc
                                | ShapeType::Cylinder
                                | ShapeType::Cone
                        );
                        if has_normal {
                            ui.label("Normal");
                            changed |= drag_vec3(ui, &mut shape.normal, 0.01, Some(-1.0..=1.0));
                        }

                        if shape.radius > 0.0 {
                            changed |= ui
                                .add(
                                    egui::Slider::new(&mut shape.radius, 0.01..=100.0)
                                        .text("Radius")
                                        .logarithmic(true),
                                )
                                .pointer()
                                .changed();
                        }

                        let has_height = matches!(
                            shape.shape_type,
                            ShapeType::Cylinder
                                | ShapeType::Cone
                                | ShapeType::Paraboloid
                                | ShapeType::Hyperboloid
                        );
                        if has_height {
                            changed |= ui
                                .add(
                                    egui::Slider::new(&mut shape.height, 0.01..=50.0)
                                        .text("Height")
                                        .logarithmic(true),
                                )
                                .pointer()
                                .changed();
                        }

                        if shape.shape_type == ShapeType::Torus {
                            changed |= ui
                                .add(
                                    egui::Slider::new(&mut shape.radius2, 0.01..=10.0)
                                        .text("Minor R")
                                        .logarithmic(true),
                                )
                                .pointer()
                                .changed();
                        }

                        // Fractal hyperparameters
                        if shape.shape_type == ShapeType::Mandelbulb {
                            changed |= ui
                                .add(
                                    egui::Slider::new(&mut shape.power, 2.0..=16.0)
                                        .text("Power")
                                        .integer(),
                                )
                                .pointer()
                                .changed();
                        }
                        if is_fractal {
                            let mut iters = shape.max_iterations as f32;
                            if ui
                                .add(
                                    egui::Slider::new(&mut iters, 1.0..=64.0)
                                        .text("Iterations")
                                        .integer(),
                                )
                                .pointer()
                                .changed()
                            {
                                shape.max_iterations = iters as u32;
                                changed = true;
                            }
                        }
                    }

                    ui.separator();
                    ui.label("Material");

                    // Each preset fully resets all material fields to avoid stale values.
                    ui.horizontal_wrapped(|ui| {
                        let mat = &mut shape.material;
                        if preset_button(ui, "Diff", "Diffuse (matte surface)") {
                            apply_preset(mat, 0.0, 0.9, 0.0, mat.ior, [0.0; 3], 0.0);
                            shape.negative = false;
                            changed = true;
                        }
                        if preset_button(ui, "Emit", "Emissive (light source)") {
                            apply_preset(mat, 0.0, 0.9, 0.0, mat.ior, [1.0; 3], 5.0);
                            shape.negative = false;
                            changed = true;
                        }
                        if preset_button(ui, "Refl", "Reflective (mirror/metal)") {
                            apply_preset(mat, 1.0, 0.05, 0.0, mat.ior, [0.0; 3], 0.0);
                            shape.negative = false;
                            changed = true;
                        }
                        if preset_button(ui, "Trans", "Transparent (clear)") {
                            apply_preset(mat, 0.0, 0.0, 1.0, 1.0, [0.0; 3], 0.0);
                            shape.negative = false;
                            changed = true;
                        }
                        if preset_button(ui, "Glass", "Glass (refractive)") {
                            apply_preset(mat, 0.0, 0.0, 1.0, 1.5, [0.0; 3], 0.0);
                            shape.negative = false;
                            changed = true;
                        }
                        if preset_button(ui, "Neg", "Negative (CSG subtraction)") {
                            shape.negative = !shape.negative;
                            changed = true;
                        }
                    });

                    let mat = &mut shape.material;

                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut color = mat.base_color;
                        if ui.color_edit_button_rgb(&mut color).pointer().changed() {
                            mat.base_color = color;
                            changed = true;
                        }
                    });

                    changed |= ui
                        .add(egui::Slider::new(&mut mat.metallic, 0.0..=1.0).text("Metallic"))
                        .pointer()
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut mat.roughness, 0.0..=1.0).text("Roughness"))
                        .pointer()
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut mat.transmission, 0.0..=1.0)
                                .text("Transmission"),
                        )
                        .pointer()
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut mat.ior, 1.0..=3.0).text("IOR"))
                        .pointer()
                        .changed();

                    if mat.emission_strength > 0.0 {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Emission:");
                            let mut color = mat.emission;
                            if ui.color_edit_button_rgb(&mut color).pointer().changed() {
                                mat.emission = color;
                                changed = true;
                            }
                        });
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut mat.emission_strength, 0.0..=50.0)
                                    .text("Strength"),
                            )
                            .pointer()
                            .changed();
                    }

                    ui.separator();
                    ui.label("Texture");

                    ui.horizontal(|ui| {
                        if ui.small_button("...").pointer().clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tga"])
                                .pick_file()
                        {
                            shape.texture = Some(path.to_string_lossy().to_string());
                            changed = true;
                            actions.textures_dirty = true;
                        }
                        if let Some(ref tex_path) = shape.texture {
                            let display_name = Path::new(tex_path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| tex_path.clone());
                            ui.label(&display_name);
                            if ui.small_button("x").pointer().clicked() {
                                shape.texture = None;
                                changed = true;
                                actions.textures_dirty = true;
                            }
                        } else {
                            ui.label("None");
                        }
                    });

                    if shape.texture.is_some() {
                        let scale = shape.texture_scale.get_or_insert(1.0);
                        changed |= ui
                            .add(
                                egui::Slider::new(scale, 0.01..=10.0)
                                    .text("Scale")
                                    .logarithmic(true),
                            )
                            .pointer()
                            .changed();
                    }

                    if changed {
                        actions.scene_dirty = true;
                    }
                });
        });
}

/// Render three DragValues for an XYZ vector, returning true if any changed.
fn drag_vec3(
    ui: &mut Ui,
    v: &mut [f32; 3],
    speed: f64,
    range: Option<std::ops::RangeInclusive<f64>>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (component, prefix) in v.iter_mut().zip(["x: ", "y: ", "z: "]) {
            let mut drag = egui::DragValue::new(component).speed(speed).prefix(prefix);
            if let Some(r) = range.clone() {
                drag = drag.range(r);
            }
            changed |= ui.add(drag).pointer().changed();
        }
    });
    changed
}

/// Render three DragValues for an XYZ rotation (degrees).
fn drag_vec3_deg(ui: &mut Ui, v: &mut [f32; 3], speed: f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (component, prefix) in v.iter_mut().zip(["x: ", "y: ", "z: "]) {
            changed |= ui
                .add(
                    egui::DragValue::new(component)
                        .speed(speed)
                        .prefix(prefix)
                        .suffix("°"),
                )
                .pointer()
                .changed();
        }
    });
    changed
}

fn preset_button(ui: &mut Ui, label: &str, tooltip: &str) -> bool {
    let response = ui.small_button(label);
    let clicked = response.clicked();
    if response.hovered() {
        egui::show_tooltip(ui.ctx(), ui.layer_id(), response.id.with("tip"), |ui| {
            ui.label(tooltip);
        });
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    clicked
}

/// Apply a material preset, resetting all PBR fields at once.
fn apply_preset(
    mat: &mut Material,
    metallic: f32,
    roughness: f32,
    transmission: f32,
    ior: f32,
    emission: [f32; 3],
    emission_strength: f32,
) {
    mat.metallic = metallic;
    mat.roughness = roughness;
    mat.transmission = transmission;
    mat.ior = ior;
    mat.emission = emission;
    mat.emission_strength = emission_strength;
}
