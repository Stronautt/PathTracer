// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Instant;

use crate::gpu::buffers;
use crate::ui;

use super::state::{AppState, FileDialogResult};

impl AppState {
    pub fn update_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.ui_state.sample_count = self.accumulator.sample_count;
        self.ui_state.render_elapsed_secs = self.accumulator.render_start.elapsed().as_secs_f32();

        let moved = self.controller.update(&mut self.camera, dt);
        let rotated = self.controller.apply_mouse_look(&mut self.camera);
        if moved || rotated {
            self.accumulator.reset();
        }

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let mut ui_actions = ui::UiActions::default();
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            ui_actions = ui::draw_ui(ctx, &mut self.ui_state, &mut self.shapes);
        });

        self.apply_ui_actions(ui_actions);

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);
        // egui's platform output may re-show the cursor; restore hidden state if needed.
        if self.controller.mouse_look_key {
            self.window.set_cursor_visible(false);
        }

        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.gpu.width(), self.gpu.height()],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let mut needs_accum_clear = false;
        if !self.ui_state.paused {
            needs_accum_clear = self.accumulator.advance();

            let gpu_camera = self.camera.to_gpu(
                self.gpu.width(),
                self.gpu.height(),
                self.frame_index,
                self.accumulator.sample_count,
            );
            buffers::update_uniform_buffer(&self.gpu.queue, &self.camera_buffer, &gpu_camera);
            self.frame_index = self.frame_index.wrapping_add(1);
        }

        let output = match self.gpu.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.gpu.resize(self.gpu.width(), self.gpu.height());
                return;
            }
            Err(e) => {
                log::error!("Surface error: {e}");
                return;
            }
        };

        // Measure FPS after GPU sync point (get_current_texture blocks when
        // the GPU is behind), so the counter reflects actual frame throughput.
        let after_acquire = Instant::now();
        let frame_dt = (after_acquire - self.last_acquire_time).as_secs_f32();
        self.last_acquire_time = after_acquire;
        self.ui_state.fps = if frame_dt > 0.0 { 1.0 / frame_dt } else { 0.0 };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if !self.ui_state.paused {
            // Clear on GPU to avoid a large CPU allocation per reset.
            if needs_accum_clear {
                encoder.clear_buffer(&self.accumulation_buffer, 0, None);
            }

            crate::render::frame::dispatch_path_trace(
                &mut encoder,
                &self.compute_pipeline,
                &[&self.compute_bind_group_0, &self.compute_bind_group_1],
                self.gpu.width(),
                self.gpu.height(),
            );

            if !self.active_effects.is_empty() {
                crate::render::frame::dispatch_post_process(
                    &mut encoder,
                    &self.post_process_pipeline,
                    &self.post_bind_group,
                    self.gpu.width(),
                    self.gpu.height(),
                );
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.set_bind_group(0, Some(&self.blit_bind_group), &[]);
            render_pass.draw(0..3, 0..1);
        }

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = render_pass.forget_lifetime();
            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Non-blocking poll: reclaim completed staging buffers without stalling the CPU.
        // VSync (PresentMode::AutoVsync) provides frame pacing.
        self.gpu.device.poll(wgpu::Maintain::Poll);

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }

    fn apply_ui_actions(&mut self, ui_actions: ui::UiActions) {
        if let Some(exp) = ui_actions.exposure_changed {
            self.camera.exposure = exp;
            self.accumulator.reset();
        }
        if let Some(effects) = ui_actions.effects_changed {
            self.active_effects = effects;
            let params = AppState::build_post_params(
                self.gpu.width(),
                self.gpu.height(),
                &self.active_effects,
            );
            buffers::update_uniform_buffer(&self.gpu.queue, &self.post_params_buffer, &params);
        }
        if let Some(shape_type) = ui_actions.shape_to_add {
            self.add_shape(shape_type);
        }
        if let Some(idx) = ui_actions.shape_to_delete {
            self.delete_shape(idx);
        }
        if ui_actions.scene_dirty {
            if ui_actions.textures_dirty {
                self.rebuild_scene_buffers_with_textures();
            } else {
                self.rebuild_scene_buffers();
            }
            self.accumulator.reset();
        }
        if ui_actions.save_requested {
            self.save_scene(&self.ui_state.save_filename.clone());
        }
        if let Some(path) = ui_actions.import_scene_path {
            self.import_scene(&path);
        }
        if let Some(path) = ui_actions.import_model_path {
            self.import_model(&path);
        }
        // Spawn file dialogs on background threads to avoid blocking the event loop.
        if ui_actions.open_import_scene_dialog {
            let tx = self.file_dialog_tx.clone();
            std::thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("YAML scene", &["yaml", "yml", "json"])
                    .pick_file()
                {
                    let _ = tx.send(FileDialogResult::ImportScene(path));
                }
            });
        }
        if ui_actions.open_import_model_dialog {
            let tx = self.file_dialog_tx.clone();
            std::thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("OBJ model", &["obj"])
                    .pick_file()
                {
                    let _ = tx.send(FileDialogResult::ImportModel(path));
                }
            });
        }
        // Poll for completed file dialog results (non-blocking).
        while let Ok(result) = self.file_dialog_rx.try_recv() {
            match result {
                FileDialogResult::ImportScene(path) => self.import_scene(&path),
                FileDialogResult::ImportModel(path) => self.import_model(&path),
            }
        }
        if let Some(path) = ui_actions.screenshot_path {
            self.take_screenshot(&path);
        }
    }

    pub fn take_screenshot(&self, path: &str) {
        let width = self.gpu.width();
        let height = self.gpu.height();
        let bytes_per_row_unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_row_padded = bytes_per_row_unpadded.div_ceil(align) * align;

        let staging_buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot staging"),
            size: (bytes_per_row_padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("screenshot encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row_padded),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.gpu.device.poll(wgpu::Maintain::Wait);

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            // Remove row padding if necessary.
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let start = (row * bytes_per_row_padded) as usize;
                let end = start + bytes_per_row_unpadded as usize;
                pixels.extend_from_slice(&data[start..end]);
            }
            drop(data);
            staging_buffer.unmap();

            if let Err(e) = crate::io::screenshot::save_screenshot(
                &pixels,
                width,
                height,
                std::path::Path::new(path),
            ) {
                log::error!("Screenshot failed: {e:#}");
            }
        } else {
            log::error!("Failed to map screenshot buffer");
        }
    }
}
