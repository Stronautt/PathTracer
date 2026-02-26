// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

use crate::constants::MODEL_AUTO_SCALE_TARGET;
use crate::scene::material::Material;
use crate::scene::scene::{CameraConfig, Scene};
use crate::scene::shape::{Shape, ShapeType};

use crate::camera::camera::Camera;

use super::state::AppState;

impl AppState {
    pub fn open_scene(&mut self, path: &Path) {
        match crate::scene::loader::load_scene(path) {
            Ok(scene) => {
                self.camera = Camera::new(
                    scene.camera.position.into(),
                    scene.camera.rotation,
                    scene.camera.fov,
                    scene.camera.exposure,
                );
                self.ui_state.exposure = self.camera.exposure;
                self.shapes = scene.shapes;

                for model_ref in &scene.models {
                    match crate::model::obj_loader::load_obj(
                        &model_ref.path,
                        model_ref.position,
                        model_ref.scale,
                        &model_ref.material,
                    ) {
                        Ok(triangles) => self.shapes.extend(triangles),
                        Err(e) => {
                            log::error!("Failed to load model '{}': {e:#}", model_ref.path)
                        }
                    }
                }

                self.ui_state.selected_shape = None;
                self.rebuild_scene_buffers_with_textures();
                self.accumulator.reset();
                log::info!("Opened scene: {}", path.display());
            }
            Err(e) => log::error!("Failed to open scene: {e:#}"),
        }
    }

    pub fn add_shape(&mut self, shape_type: ShapeType) {
        let mut shape = Shape {
            name: None,
            shape_type,
            negative: false,
            position: self.camera.position.into(),
            normal: [0.0, 1.0, 0.0],
            radius: 1.0,
            radius2: 0.3,
            height: 2.0,
            rotation: [0.0, 0.0, 0.0],
            v0: [0.0, 0.0, 0.0],
            v1: [1.0, 0.0, 0.0],
            v2: [0.0, 1.0, 0.0],
            power: 8.0,
            max_iterations: 12,
            texture: None,
            texture_scale: None,
            uv0: [0.0, 0.0],
            uv1: [0.0, 0.0],
            uv2: [0.0, 0.0],
            material: Material::default(),
        };

        let (_, _, forward) = self.camera.basis_vectors();
        let spawn_pos = self.camera.position + forward * 5.0;
        shape.position = spawn_pos.into();

        match shape_type {
            ShapeType::Plane => {
                shape.position = [0.0, 0.0, 0.0];
            }
            ShapeType::Mandelbulb => {
                shape.radius = 1.5;
                shape.power = 8.0;
                shape.max_iterations = 12;
            }
            ShapeType::Julia => {
                shape.radius = 1.5;
                shape.rotation = [-0.8, 0.156, 0.0]; // Julia C.xyz
                shape.radius2 = -0.046; // Julia C.w
                shape.max_iterations = 14;
            }
            _ => {}
        }

        self.shapes.push(shape);
        self.rebuild_scene_buffers();
        self.accumulator.reset();
        log::info!("Added {:?} shape", shape_type);
    }

    pub fn delete_shape(&mut self, idx: usize) {
        if idx < self.shapes.len() {
            self.shapes.remove(idx);
            if let Some(sel) = self.ui_state.selected_shape {
                if sel == idx {
                    self.ui_state.selected_shape = None;
                } else if sel > idx {
                    self.ui_state.selected_shape = Some(sel - 1);
                }
            }
            self.rebuild_scene_buffers();
            self.accumulator.reset();
            log::info!("Deleted shape at index {}", idx);
        }
    }

    pub fn save_scene(&self, filename: &str) {
        let scene = Scene {
            camera: CameraConfig {
                position: self.camera.position.into(),
                rotation: [self.camera.pitch, self.camera.yaw, 0.0],
                fov: self.camera.fov,
                exposure: self.camera.exposure,
            },
            shapes: self.shapes.clone(),
            models: vec![],
        };
        if let Err(e) = crate::scene::exporter::save_scene(&scene, Path::new(filename)) {
            log::error!("Failed to save scene: {e:#}");
        }
    }

    pub fn import_scene(&mut self, path: &Path) {
        match crate::scene::loader::load_scene(path) {
            Ok(scene) => {
                let mut count = scene.shapes.len();
                self.shapes.extend(scene.shapes);
                for model_ref in &scene.models {
                    match crate::model::obj_loader::load_obj(
                        &model_ref.path,
                        model_ref.position,
                        model_ref.scale,
                        &model_ref.material,
                    ) {
                        Ok(triangles) => {
                            count += triangles.len();
                            self.shapes.extend(triangles);
                        }
                        Err(e) => {
                            log::error!("Failed to load model '{}': {e:#}", model_ref.path)
                        }
                    }
                }
                self.rebuild_scene_buffers_with_textures();
                self.accumulator.reset();
                log::info!("Imported {} shapes from {}", count, path.display());
            }
            Err(e) => log::error!("Failed to import scene: {e:#}"),
        }
    }

    pub fn import_model(&mut self, path: &Path) {
        let path_str = path.to_string_lossy();

        let (_, _, forward) = self.camera.basis_vectors();
        let spawn_distance = MODEL_AUTO_SCALE_TARGET * 2.0;
        let position: [f32; 3] = (self.camera.position + forward * spawn_distance).into();

        match crate::model::obj_loader::load_obj_auto_scaled(
            &path_str,
            position,
            MODEL_AUTO_SCALE_TARGET,
            &Material::default(),
        ) {
            Ok(triangles) => {
                let count = triangles.len();
                self.shapes.extend(triangles);
                self.rebuild_scene_buffers_with_textures();
                self.accumulator.reset();
                log::info!("Imported {} triangles from {}", count, path.display());
            }
            Err(e) => log::error!("Failed to import model: {e:#}"),
        }
    }
}
