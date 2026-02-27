// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use egui::Context;

use super::{Pointer, UiActions, UiState, shape_label};
use crate::constants::{EXAMPLE_SCENES_DIR, resolve_data_path};
use crate::render::post_process::PostEffect;
use crate::scene::shape::{Shape, ShapeType};

/// Render a labelled slider and set `*changed = true` when the value is modified.
fn labeled_slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.add(egui::Slider::new(value, range)).pointer().changed() {
            *changed = true;
        }
    });
}

/// Like `labeled_slider` but indented by `indent` points — used for effect sub-options.
fn indented_slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    indent: f32,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.add_space(indent);
        ui.label(label);
        if ui.add(egui::Slider::new(value, range)).pointer().changed() {
            *changed = true;
        }
    });
}

pub fn draw_toolbar(ctx: &Context, state: &mut UiState, shapes: &[Shape], actions: &mut UiActions) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            if ui
                .button(if state.paused {
                    "▶ Resume"
                } else {
                    "⏸ Pause"
                })
                .pointer()
                .clicked()
            {
                state.paused = !state.paused;
            }
            actions.paused = state.paused;

            ui.separator();

            ui.menu_button("🎬 Scene", |ui| {
                if ui.button("📂 Open...").pointer().clicked() {
                    actions.open_scene_dialog = true;
                    ui.close_menu();
                }
                if ui.button("💾 Save...").pointer().clicked() {
                    state.save_dialog_open = true;
                    ui.close_menu();
                }
                if ui.button("📷 Screenshot").pointer().clicked() {
                    state.screenshot_filename = crate::io::screenshot::default_screenshot_path()
                        .to_string_lossy()
                        .to_string();
                    state.screenshot_dialog_open = true;
                    ui.close_menu();
                }

                ui.separator();

                ui.menu_button("📂 Import...", |ui| {
                    if ui.button("Scene (.yaml)").pointer().clicked() {
                        actions.open_import_scene_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("3D Model (.obj)").pointer().clicked() {
                        actions.open_import_model_dialog = true;
                        ui.close_menu();
                    }
                })
                .response
                .pointer();

                ui.menu_button("📁 Examples", |ui| {
                    if state.example_scenes.is_empty() {
                        ui.disable();
                        ui.label("No examples found");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                for name in &state.example_scenes {
                                    if ui.button(name).pointer().clicked() {
                                        let full = resolve_data_path(EXAMPLE_SCENES_DIR)
                                            .join(format!("{name}.yaml"));
                                        actions.open_example_scene = Some(full);
                                        ui.close_menu();
                                    }
                                }
                            });
                    }
                })
                .response
                .pointer();

                ui.menu_button("➕ Add Shape", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            ui.strong("Elementary");
                            for &shape_type in ShapeType::ELEMENTARY {
                                if ui.button(shape_type.label()).pointer().clicked() {
                                    actions.shape_to_add = Some(shape_type);
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            ui.strong("Complex");
                            for &shape_type in ShapeType::COMPLEX {
                                if ui.button(shape_type.label()).pointer().clicked() {
                                    actions.shape_to_add = Some(shape_type);
                                    ui.close_menu();
                                }
                            }
                        });
                })
                .response
                .pointer();

                ui.separator();

                ui.strong("Shapes");
                if shapes.is_empty() {
                    ui.label("No shapes in scene");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            draw_shapes_list(ui, shapes, state, actions);
                        });
                }
            })
            .response
            .pointer();

            ui.menu_button("⚙ Settings", |ui| {
                ui.set_min_width(200.0);

                ui.horizontal(|ui| {
                    ui.label("Exposure:");
                    if ui
                        .add(egui::Slider::new(&mut state.exposure, 0.1..=10.0).logarithmic(true))
                        .pointer()
                        .changed()
                    {
                        actions.exposure_changed = Some(state.exposure);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Max Bounces:");
                    if ui
                        .add(egui::Slider::new(&mut state.max_bounces, 1..=32))
                        .pointer()
                        .changed()
                    {
                        actions.max_bounces_changed = Some(state.max_bounces);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Firefly Clamp:");
                    if ui
                        .add(
                            egui::Slider::new(&mut state.firefly_clamp, 1.0..=1000.0)
                                .logarithmic(true),
                        )
                        .pointer()
                        .changed()
                    {
                        actions.render_settings_changed = true;
                    }
                });

                labeled_slider(
                    ui,
                    "Fractal Steps:",
                    &mut state.fractal_march_steps,
                    32..=512,
                    &mut actions.render_settings_changed,
                );

                ui.horizontal(|ui| {
                    ui.label("Tone Mapper:");
                    let labels = ["ACES", "Reinhard", "None"];
                    let current = labels.get(state.tone_mapper as usize).unwrap_or(&"ACES");
                    egui::ComboBox::from_id_salt("tone_mapper")
                        .selected_text(*current)
                        .show_ui(ui, |ui| {
                            for (i, label) in labels.iter().enumerate() {
                                if ui
                                    .selectable_value(&mut state.tone_mapper, i as u32, *label)
                                    .pointer()
                                    .changed()
                                {
                                    actions.render_settings_changed = true;
                                }
                            }
                        });
                });

                ui.separator();
                ui.strong("Skybox");

                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let mut color = state.skybox_color;
                    if ui.color_edit_button_rgb(&mut color).pointer().changed() {
                        state.skybox_color = color;
                        actions.render_settings_changed = true;
                    }
                });

                labeled_slider(
                    ui,
                    "Brightness:",
                    &mut state.skybox_brightness,
                    0.0..=2.0,
                    &mut actions.render_settings_changed,
                );

                ui.separator();

                ui.strong("Effects");
                let mut effects_changed = false;
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for &effect in PostEffect::ALL_EFFECTS {
                            let active = state.active_effects.contains(&effect);
                            let mut checked = active;
                            if ui
                                .checkbox(&mut checked, effect.label())
                                .pointer()
                                .clicked()
                            {
                                if checked {
                                    state.active_effects.push(effect);
                                } else {
                                    state.active_effects.retain(|&e| e != effect);
                                }
                                effects_changed = true;
                            }
                            if checked && effect == PostEffect::OilPainting {
                                indented_slider(
                                    ui,
                                    20.0,
                                    "Radius:",
                                    &mut state.oil_radius,
                                    1..=8,
                                    &mut actions.post_effect_params_changed,
                                );
                            }
                            if checked && effect == PostEffect::Comic {
                                indented_slider(
                                    ui,
                                    20.0,
                                    "Levels:",
                                    &mut state.comic_levels,
                                    2..=16,
                                    &mut actions.post_effect_params_changed,
                                );
                            }
                        }

                        if state.active_effects.len() >= 2 {
                            ui.separator();
                            ui.strong("Order");
                            let mut swap: Option<(usize, usize)> = None;
                            for i in 0..state.active_effects.len() {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{}. {}",
                                        i + 1,
                                        state.active_effects[i].label()
                                    ));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if i + 1 < state.active_effects.len()
                                                && ui.small_button("Dn").pointer().clicked()
                                            {
                                                swap = Some((i, i + 1));
                                            }
                                            if i > 0 && ui.small_button("Up").pointer().clicked() {
                                                swap = Some((i, i - 1));
                                            }
                                        },
                                    );
                                });
                            }
                            if let Some((a, b)) = swap {
                                state.active_effects.swap(a, b);
                                effects_changed = true;
                            }
                        }
                    });
                if effects_changed {
                    actions.effects_changed = Some(state.active_effects.clone());
                }
            })
            .response
            .pointer();

            ui.menu_button("? Help", |ui| {
                if ui.button("Shortcuts").pointer().clicked() {
                    state.shortcuts_dialog_open = true;
                    ui.close_menu();
                }
                if ui.button("About").pointer().clicked() {
                    state.about_dialog_open = true;
                    ui.close_menu();
                }
            })
            .response
            .pointer();

            ui.separator();

            ui.label(format!("FPS: {:.0}", state.fps));
            ui.label(format!("Samples: {}", state.sample_count));
            ui.label(format!(
                "Time: {}",
                format_elapsed(state.render_elapsed_secs)
            ));
        });
    });
}

fn format_elapsed(secs: f32) -> String {
    let mins = (secs / 60.0) as u32;
    let remaining = secs % 60.0;
    format!("{mins}:{remaining:05.2}")
}

/// Draw the shapes list, collapsing consecutive same-named shapes into groups.
fn draw_shapes_list(
    ui: &mut egui::Ui,
    shapes: &[Shape],
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let mut i = 0;
    while i < shapes.len() {
        // Check if this starts a run of shapes with the same non-empty name.
        let name = shapes[i].name.as_deref().unwrap_or("");
        if !name.is_empty() {
            let group_start = i;
            let mut group_end = i + 1;
            while group_end < shapes.len() && shapes[group_end].name.as_deref() == Some(name) {
                group_end += 1;
            }
            let count = group_end - group_start;

            if count > 1 {
                // Render as a collapsible group.
                let header = format!("{name} ({count})");
                egui::CollapsingHeader::new(&header)
                    .default_open(false)
                    .show(ui, |ui| {
                        for j in group_start..group_end {
                            draw_group_child_entry(ui, shapes, j, state, actions);
                        }
                    });
                i = group_end;
                continue;
            }
        }

        // Single (ungrouped) shape.
        draw_shape_entry(ui, shapes, i, state, actions);
        i += 1;
    }
}

/// Entry for a child within a collapsible group — shows "Type #idx" instead of the group name.
fn draw_group_child_entry(
    ui: &mut egui::Ui,
    shapes: &[Shape],
    i: usize,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let label = format!("{} #{}", shapes[i].shape_type.label(), i);
    draw_selectable_shape_entry(ui, i, &label, state, actions);
}

fn draw_shape_entry(
    ui: &mut egui::Ui,
    shapes: &[Shape],
    i: usize,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let label = shape_label(&shapes[i], i);
    draw_selectable_shape_entry(ui, i, &label, state, actions);
}

fn draw_selectable_shape_entry(
    ui: &mut egui::Ui,
    i: usize,
    label: &str,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let selected = state.selected_shape == Some(i);
    ui.horizontal(|ui| {
        let response = ui.selectable_label(selected, label).pointer();
        if ui.small_button("x").pointer().clicked() {
            state.confirm_delete_shape = Some(i);
        }
        if response.clicked() {
            state.selected_shape = Some(i);
            state.model_scale = 1.0;
            actions.selected_shape = Some(i);
            ui.close_menu();
        }
    });
}
