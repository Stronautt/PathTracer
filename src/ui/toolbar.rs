use egui::Context;

use super::{UiActions, UiState};
use crate::render::post_process::PostEffect;
use crate::scene::figure::{Figure, FigureType};

pub fn draw_toolbar(
    ctx: &Context,
    state: &mut UiState,
    figures: &[Figure],
    actions: &mut UiActions,
) {
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            if ui
                .button(if state.paused {
                    "▶ Resume"
                } else {
                    "⏸ Pause"
                })
                .clicked()
            {
                state.paused = !state.paused;
            }
            actions.paused = state.paused;

            ui.separator();

            ui.menu_button("🎬 Scene", |ui| {
                if ui.button("📷 Screenshot").clicked() {
                    actions.screenshot_requested = true;
                    ui.close_menu();
                }
                if ui.button("💾 Save").clicked() {
                    actions.save_requested = true;
                    ui.close_menu();
                }

                ui.separator();

                ui.menu_button("➕ Add Figure", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for &fig_type in FigureType::ALL {
                                if ui.button(fig_type.label()).clicked() {
                                    actions.figure_to_add = Some(fig_type);
                                    ui.close_menu();
                                }
                            }
                        });
                });

                ui.separator();

                ui.strong("Figures");
                if figures.is_empty() {
                    ui.label("No figures in scene");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.set_max_width(250.0);
                            for (i, fig) in figures.iter().enumerate() {
                                let selected = state.selected_figure == Some(i);
                                let label = format!("{} #{}", fig.figure_type.label(), i);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected, &label).clicked() {
                                        state.selected_figure = Some(i);
                                        actions.selected_figure = Some(i);
                                        ui.close_menu();
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("x").clicked() {
                                                actions.figure_to_delete = Some(i);
                                            }
                                        },
                                    );
                                });
                            }
                        });
                }
            });

            ui.menu_button("⚙ Settings", |ui| {
                ui.set_min_width(200.0);

                ui.horizontal(|ui| {
                    ui.label("Exposure:");
                    if ui
                        .add(egui::Slider::new(&mut state.exposure, 0.1..=10.0).logarithmic(true))
                        .changed()
                    {
                        actions.exposure_changed = Some(state.exposure);
                    }
                });

                ui.separator();

                ui.strong("Effects");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for &effect in PostEffect::ALL {
                            let selected = state.active_effect == effect;
                            if ui.selectable_label(selected, effect.label()).clicked() {
                                state.active_effect = effect;
                                actions.effect_changed = Some(effect);
                                ui.close_menu();
                            }
                        }
                    });
            });

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
