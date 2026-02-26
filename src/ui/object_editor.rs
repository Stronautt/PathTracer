use egui::{Context, Ui};

use super::{UiActions, UiState};
use crate::scene::figure::{Figure, FigureType};
use crate::scene::material::Material;

pub fn draw_object_editor(
    ctx: &Context,
    state: &mut UiState,
    figure: &mut Figure,
    figure_idx: usize,
    actions: &mut UiActions,
) {
    egui::SidePanel::right("object_editor")
        .min_width(200.0)
        .max_width(240.0)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            let mut changed = false;

            ui.horizontal(|ui| {
                ui.strong(format!("{} #{}", figure.figure_type.label(), figure_idx));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("x").clicked() {
                        state.selected_figure = None;
                    }
                });
            });

            ui.separator();

            ui.label("Position");
            changed |= drag_vec3(ui, &mut figure.position, 0.1, None);

            ui.label("Rotation");
            changed |= drag_vec3_deg(ui, &mut figure.rotation, 1.0);

            let has_normal = matches!(
                figure.figure_type,
                FigureType::Plane | FigureType::Disc | FigureType::Cylinder | FigureType::Cone
            );
            if has_normal {
                ui.label("Normal");
                changed |= drag_vec3(ui, &mut figure.normal, 0.01, Some(-1.0..=1.0));
            }

            if figure.radius > 0.0 {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut figure.radius, 0.01..=100.0)
                            .text("Radius")
                            .logarithmic(true),
                    )
                    .changed();
            }

            let has_height = matches!(
                figure.figure_type,
                FigureType::Cylinder
                    | FigureType::Cone
                    | FigureType::Paraboloid
                    | FigureType::Hyperboloid
            );
            if has_height {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut figure.height, 0.01..=50.0)
                            .text("Height")
                            .logarithmic(true),
                    )
                    .changed();
            }

            if figure.figure_type == FigureType::Torus {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut figure.radius2, 0.01..=10.0)
                            .text("Minor R")
                            .logarithmic(true),
                    )
                    .changed();
            }

            ui.separator();
            ui.label("Material");

            // Each preset fully resets all material fields to avoid stale values.
            ui.horizontal_wrapped(|ui| {
                let mat = &mut figure.material;
                if preset_button(ui, "Diffuse") {
                    apply_preset(mat, 0.0, 0.9, 0.0, mat.ior, [0.0; 3], 0.0);
                    changed = true;
                }
                if preset_button(ui, "Emissive") {
                    apply_preset(mat, 0.0, 0.9, 0.0, mat.ior, [1.0; 3], 5.0);
                    changed = true;
                }
                if preset_button(ui, "Reflect") {
                    apply_preset(mat, 1.0, 0.05, 0.0, mat.ior, [0.0; 3], 0.0);
                    changed = true;
                }
                if preset_button(ui, "Transparent") {
                    apply_preset(mat, 0.0, 0.0, 1.0, 1.0, [0.0; 3], 0.0);
                    changed = true;
                }
                if preset_button(ui, "Glass") {
                    apply_preset(mat, 0.0, 0.0, 1.0, 1.5, [0.0; 3], 0.0);
                    changed = true;
                }
                if preset_button(ui, "Negative") {
                    mat.base_color = [0.0; 3];
                    apply_preset(mat, 0.0, 1.0, 0.0, mat.ior, [0.0; 3], 0.0);
                    changed = true;
                }
            });

            let mat = &mut figure.material;

            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color = mat.base_color;
                if ui.color_edit_button_rgb(&mut color).changed() {
                    mat.base_color = color;
                    changed = true;
                }
            });

            changed |= ui
                .add(egui::Slider::new(&mut mat.metallic, 0.0..=1.0).text("Metallic"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut mat.roughness, 0.0..=1.0).text("Roughness"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut mat.transmission, 0.0..=1.0).text("Transmission"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut mat.ior, 1.0..=3.0).text("IOR"))
                .changed();

            if mat.emission_strength > 0.0 {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Emission:");
                    let mut color = mat.emission;
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        mat.emission = color;
                        changed = true;
                    }
                });
                changed |= ui
                    .add(egui::Slider::new(&mut mat.emission_strength, 0.0..=50.0).text("Strength"))
                    .changed();
            }

            if changed {
                actions.scene_dirty = true;
            }
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
            changed |= ui.add(drag).changed();
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
                .changed();
        }
    });
    changed
}

fn preset_button(ui: &mut Ui, label: &str) -> bool {
    ui.small_button(label).clicked()
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
