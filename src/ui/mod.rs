pub mod object_editor;
pub mod toolbar;

use egui::Context;

use crate::render::post_process::PostEffect;
use crate::scene::figure::{Figure, FigureType};

#[derive(Default)]
pub struct UiActions {
    pub screenshot_requested: bool,
    pub save_requested: bool,
    pub paused: bool,
    pub exposure_changed: Option<f32>,
    pub effect_changed: Option<PostEffect>,
    pub figure_to_add: Option<FigureType>,
    pub selected_figure: Option<usize>,
    pub scene_dirty: bool,
    pub figure_to_delete: Option<usize>,
}

pub struct UiState {
    pub paused: bool,
    pub active_effect: PostEffect,
    pub exposure: f32,
    pub selected_figure: Option<usize>,
    pub fps: f32,
    pub sample_count: u32,
    pub render_elapsed_secs: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: false,
            active_effect: PostEffect::None,
            exposure: 1.0,
            selected_figure: None,
            fps: 0.0,
            sample_count: 0,
            render_elapsed_secs: 0.0,
        }
    }
}

pub fn draw_ui(ctx: &Context, state: &mut UiState, figures: &mut [Figure]) -> UiActions {
    let mut actions = UiActions::default();

    toolbar::draw_toolbar(ctx, state, figures, &mut actions);

    if let Some(idx) = state.selected_figure
        && idx < figures.len()
    {
        object_editor::draw_object_editor(ctx, state, &mut figures[idx], idx, &mut actions);
    }

    actions
}
