// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later
//
// App module — split into focused submodules:
//   app/state.rs       — AppState struct, initialization, GPU state management
//   app/rendering.rs   — Render dispatch, frame loop, UI actions
//   app/scene_ops.rs   — Scene loading/saving, shape management, OBJ import
//   app/interaction.rs — Object picking, dragging, window/mouse event handling

#[path = "app/interaction.rs"]
mod interaction;
#[path = "app/rendering.rs"]
mod rendering;
#[path = "app/scene_ops.rs"]
mod scene_ops;
#[path = "app/state.rs"]
mod state;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

pub use state::AppState;

pub fn run(scene_path: Option<String>) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(scene_path);
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    scene_path: Option<String>,
    state: Option<AppState>,
}

impl App {
    fn new(scene_path: Option<String>) -> Self {
        Self {
            scene_path,
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        match AppState::new(event_loop, &self.scene_path) {
            Ok(state) => self.state = Some(state),
            Err(e) => {
                log::error!("Failed to initialize: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            interaction::handle_window_event(state, event_loop, event);
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(state) = &mut self.state
            && let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
        {
            state.controller.accumulate_raw_delta(dx, dy);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}
