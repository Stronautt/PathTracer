// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use anyhow::Result;
use bytemuck::Zeroable;
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window};

use crate::accel::aabb::shape_aabb;
use crate::accel::bvh::Bvh;
use crate::camera::camera::Camera;
use crate::camera::controller::CameraController;
use crate::constants::*;
use crate::gpu::buffers;
use crate::gpu::context::GpuContext;
use crate::io::texture_atlas::TextureAtlas;
use crate::render::accumulator::Accumulator;
use crate::render::post_process::PostEffect;
use crate::scene::material::GpuMaterial;
use crate::scene::scene::Scene;
use crate::scene::shape::{GpuShape, Shape, ShapeType};
use crate::shaders::composer::ShaderComposer;
use crate::ui;

pub enum FileDialogResult {
    OpenScene(PathBuf),
    ImportScene(PathBuf),
    ImportModel(PathBuf),
    Screenshot(PathBuf),
}

pub struct AppState {
    pub window: Arc<Window>,
    pub file_dialog_rx: mpsc::Receiver<FileDialogResult>,
    pub file_dialog_tx: mpsc::Sender<FileDialogResult>,
    pub gpu: GpuContext,
    pub scene: Scene,
    pub shapes: Vec<Shape>,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub blit_pipeline: wgpu::RenderPipeline,
    pub post_process_pipeline: wgpu::ComputePipeline,
    pub camera_buffer: wgpu::Buffer,
    pub accumulation_buffer: wgpu::Buffer,
    pub shape_buffer: wgpu::Buffer,
    pub material_buffer: wgpu::Buffer,
    pub bvh_node_buffer: wgpu::Buffer,
    pub bvh_prim_buffer: wgpu::Buffer,
    pub light_index_buffer: wgpu::Buffer,
    pub infinite_index_buffer: wgpu::Buffer,
    pub infinite_indices: Vec<u32>,
    pub tex_pixels_buffer: wgpu::Buffer,
    pub tex_infos_buffer: wgpu::Buffer,
    pub texture_atlas: TextureAtlas,
    pub tex_path_cache: HashMap<String, i32>,
    pub output_texture: wgpu::Texture,
    pub output_view: wgpu::TextureView,
    pub compute_bind_group_0: wgpu::BindGroup,
    pub compute_bind_group_1: wgpu::BindGroup,
    pub blit_bind_group: wgpu::BindGroup,
    pub post_bind_group: wgpu::BindGroup,
    pub compute_bg_layout_0: wgpu::BindGroupLayout,
    pub compute_bg_layout_1: wgpu::BindGroupLayout,
    pub blit_bg_layout: wgpu::BindGroupLayout,
    pub post_bg_layout: wgpu::BindGroupLayout,
    pub post_params_buffer: wgpu::Buffer,
    pub blit_sampler: wgpu::Sampler,
    pub bvh: Bvh,
    pub camera: Camera,
    pub controller: CameraController,
    pub accumulator: Accumulator,
    pub drag_shape: Option<usize>,
    pub drag_depth: f32,
    pub drag_offset: glam::Vec3,
    pub drag_moved: bool,
    pub drag_start_pos: (f32, f32),
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub ui_state: ui::UiState,
    pub last_frame: Instant,
    pub last_acquire_time: Instant,
    pub frame_index: u32,
    pub active_effects: Vec<PostEffect>,
}

impl AppState {
    pub fn new(event_loop: &ActiveEventLoop, scene_path: &Option<String>) -> Result<Self> {
        let mut attrs = Window::default_attributes()
            .with_title("PathTracer")
            .with_inner_size(PhysicalSize::new(
                DEFAULT_WINDOW_WIDTH,
                DEFAULT_WINDOW_HEIGHT,
            ));

        if let Ok(img) = image::open(crate::constants::resolve_data_path(WINDOW_ICON_PATH)) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            if let Ok(icon) = Icon::from_rgba(rgba.into_raw(), w, h) {
                attrs = attrs.with_window_icon(Some(icon));
            }
        }

        let window = Arc::new(event_loop.create_window(attrs)?);
        let gpu = GpuContext::new(window.clone())?;
        let width = gpu.width();
        let height = gpu.height();

        let scene = if let Some(path) = scene_path {
            crate::scene::loader::load_scene(Path::new(path))?
        } else {
            Scene::empty()
        };

        let camera = Camera::from_config(&scene.camera);

        let mut shapes = scene.shapes.clone();
        for model_ref in &scene.models {
            match crate::model::obj_loader::load_obj(
                &model_ref.path,
                model_ref.position,
                model_ref.scale,
                &model_ref.material,
            ) {
                Ok(triangles) => {
                    log::info!(
                        "Loaded model '{}': {} triangles",
                        model_ref.path,
                        triangles.len()
                    );
                    shapes.extend(triangles);
                }
                Err(e) => log::error!("Failed to load model '{}': {e:#}", model_ref.path),
            }
        }

        let (texture_atlas, tex_path_cache) = Self::build_texture_atlas(&shapes);
        let (gpu_shapes, gpu_materials, light_indices) =
            Self::build_gpu_data(&shapes, &tex_path_cache);

        let (bvh, infinite_indices) = Self::build_bvh(&shapes);

        let composer = ShaderComposer::from_directory(&ShaderComposer::shader_dir())?;
        let trace_source = composer.compose("path_trace")?;
        let blit_source = composer.compose("blit")?;
        let post_source = composer.compose("post_process")?;

        let gpu_camera = camera.to_gpu(width, height, 0, 0);
        let camera_buffer = buffers::create_uniform_buffer(&gpu.device, &gpu_camera, "camera");

        let accum_size = (width * height) as u64 * ACCUM_BYTES_PER_PIXEL;
        let accumulation_buffer =
            buffers::create_empty_storage_buffer(&gpu.device, accum_size, "accumulation");

        let (output_texture, output_view) =
            buffers::create_output_texture(&gpu.device, width, height, "output");

        let (
            shape_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
            infinite_index_buffer,
        ) = Self::create_geometry_buffers(
            &gpu.device,
            &gpu_shapes,
            &gpu_materials,
            &bvh,
            &light_indices,
            &infinite_indices,
        );

        let tex_pixels_buffer =
            buffers::create_storage_buffer(&gpu.device, &texture_atlas.pixels, "tex_pixels", true);
        let tex_infos_buffer =
            buffers::create_storage_buffer(&gpu.device, &texture_atlas.infos, "tex_infos", true);

        let post_params =
            Self::build_post_params(width, height, &[], DEFAULT_OIL_RADIUS, DEFAULT_COMIC_LEVELS);
        let post_params_buffer =
            buffers::create_uniform_buffer(&gpu.device, &post_params, "post_params");

        let compute_bg_layout_0 = Self::create_compute_bg0_layout(&gpu.device);
        let compute_bg_layout_1 = Self::create_compute_bg1_layout(&gpu.device);
        let blit_bg_layout = Self::create_blit_bg_layout(&gpu.device);
        let post_bg_layout = Self::create_post_bg_layout(&gpu.device);

        let compute_pipeline = crate::gpu::pipeline::create_compute_pipeline(
            &gpu.device,
            &trace_source,
            &[&compute_bg_layout_0, &compute_bg_layout_1],
            "path trace",
        )?;

        let blit_pipeline = crate::gpu::pipeline::create_blit_pipeline(
            &gpu.device,
            &blit_source,
            gpu.surface_format(),
            &blit_bg_layout,
        )?;

        let post_process_pipeline = crate::gpu::pipeline::create_compute_pipeline(
            &gpu.device,
            &post_source,
            &[&post_bg_layout],
            "post process",
        )?;

        let compute_bind_group_0 = Self::create_compute_bg0(
            &gpu.device,
            &compute_bg_layout_0,
            &camera_buffer,
            &accumulation_buffer,
            &output_view,
        );

        let compute_bind_group_1 = Self::create_compute_bg1(
            &gpu.device,
            &compute_bg_layout_1,
            &shape_buffer,
            &material_buffer,
            &bvh_node_buffer,
            &bvh_prim_buffer,
            &light_index_buffer,
            &tex_pixels_buffer,
            &tex_infos_buffer,
            &infinite_index_buffer,
        );

        let blit_sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let blit_bind_group =
            Self::create_blit_bind_group(&gpu.device, &blit_bg_layout, &output_view, &blit_sampler);
        let post_bind_group = Self::create_post_bind_group(
            &gpu.device,
            &post_bg_layout,
            &post_params_buffer,
            &accumulation_buffer,
            &output_view,
        );

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.surface_format(), None, 1, false);

        let mut ui_state = ui::UiState {
            paused: shapes.is_empty(),
            example_scenes: crate::constants::discover_example_scenes(),
            ..Default::default()
        };
        ui_state.sync_from_camera(&camera);

        let (file_dialog_tx, file_dialog_rx) = mpsc::channel();

        Ok(Self {
            window,
            file_dialog_rx,
            file_dialog_tx,
            gpu,
            scene,
            shapes,
            compute_pipeline,
            blit_pipeline,
            post_process_pipeline,
            camera_buffer,
            accumulation_buffer,
            shape_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
            infinite_index_buffer,
            infinite_indices,
            tex_pixels_buffer,
            tex_infos_buffer,
            texture_atlas,
            tex_path_cache,
            output_texture,
            output_view,
            compute_bind_group_0,
            compute_bind_group_1,
            blit_bind_group,
            post_bind_group,
            compute_bg_layout_0,
            compute_bg_layout_1,
            blit_bg_layout,
            post_bg_layout,
            post_params_buffer,
            blit_sampler,
            bvh,
            camera,
            controller: CameraController::new(),
            accumulator: Accumulator::default(),
            drag_shape: None,
            drag_depth: 0.0,
            drag_offset: glam::Vec3::ZERO,
            drag_moved: false,
            drag_start_pos: (0.0, 0.0),
            egui_ctx,
            egui_state,
            egui_renderer,
            ui_state,
            last_frame: Instant::now(),
            last_acquire_time: Instant::now(),
            frame_index: 0,
            active_effects: Vec::new(),
        })
    }

    pub fn build_texture_atlas(shapes: &[Shape]) -> (TextureAtlas, HashMap<String, i32>) {
        let mut atlas = TextureAtlas::new();
        let mut cache: HashMap<String, i32> = HashMap::new();

        for shape in shapes {
            if let Some(ref tex_path) = shape.texture
                && !cache.contains_key(tex_path)
            {
                match atlas.load_texture(Path::new(tex_path)) {
                    Ok(id) => {
                        cache.insert(tex_path.clone(), id as i32);
                    }
                    Err(e) => {
                        log::warn!("Failed to load texture '{}': {e:#}", tex_path);
                    }
                }
            }
        }

        (atlas, cache)
    }

    pub fn build_gpu_data(
        shapes: &[Shape],
        tex_cache: &HashMap<String, i32>,
    ) -> (Vec<GpuShape>, Vec<GpuMaterial>, Vec<u32>) {
        let mut gpu_shapes = Vec::with_capacity(shapes.len());
        let mut gpu_materials = Vec::with_capacity(shapes.len());
        let mut light_indices = Vec::new();

        for (i, shape) in shapes.iter().enumerate() {
            let mut mat = GpuMaterial::from(&shape.material);

            if let Some(ref tex_path) = shape.texture
                && let Some(&id) = tex_cache.get(tex_path)
            {
                mat.texture_id = id;
            }

            let mat_idx = gpu_materials.len() as u32;
            gpu_materials.push(mat);
            gpu_shapes.push(GpuShape::from_shape(shape, mat_idx));

            if shape.material.is_emissive() {
                light_indices.push(i as u32);
            }
        }

        (gpu_shapes, gpu_materials, light_indices)
    }

    /// wgpu requires non-empty buffers. When the list is empty, a single
    /// sentinel value (0xFFFFFFFF) is uploaded so the shader can detect it.
    fn nonempty_index_buffer(indices: &[u32]) -> &[u32] {
        if indices.is_empty() {
            &[0xFFFFFFFF]
        } else {
            indices
        }
    }

    pub fn create_geometry_buffers(
        device: &wgpu::Device,
        gpu_shapes: &[GpuShape],
        gpu_materials: &[GpuMaterial],
        bvh: &Bvh,
        light_indices: &[u32],
        infinite_indices: &[u32],
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
    ) {
        let shape_buffer = if gpu_shapes.is_empty() {
            buffers::create_storage_buffer(device, &[GpuShape::zeroed()], "shapes", true)
        } else {
            buffers::create_storage_buffer(device, gpu_shapes, "shapes", true)
        };

        let material_buffer = if gpu_materials.is_empty() {
            buffers::create_storage_buffer(device, &[GpuMaterial::zeroed()], "materials", true)
        } else {
            buffers::create_storage_buffer(device, gpu_materials, "materials", true)
        };

        let bvh_node_buffer = buffers::create_storage_buffer(device, &bvh.nodes, "bvh_nodes", true);

        let bvh_prim_buffer = if bvh.prim_indices.is_empty() {
            buffers::create_storage_buffer(device, &[0u32], "bvh_prims", true)
        } else {
            buffers::create_storage_buffer(device, &bvh.prim_indices, "bvh_prims", true)
        };

        let light_index_buffer = buffers::create_storage_buffer(
            device,
            Self::nonempty_index_buffer(light_indices),
            "light_indices",
            true,
        );

        let infinite_index_buffer = buffers::create_storage_buffer(
            device,
            Self::nonempty_index_buffer(infinite_indices),
            "infinite_indices",
            true,
        );

        (
            shape_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
            infinite_index_buffer,
        )
    }

    pub fn build_post_params(
        width: u32,
        height: u32,
        effects: &[PostEffect],
        oil_radius: u32,
        comic_levels: u32,
    ) -> [u32; POST_PARAMS_SIZE] {
        let mut params = [0u32; POST_PARAMS_SIZE];
        params[0] = width;
        params[1] = height;
        let count = effects.len().min(POST_PARAMS_MAX_EFFECTS);
        params[2] = count as u32;
        params[3] = oil_radius;
        for (i, effect) in effects.iter().take(POST_PARAMS_MAX_EFFECTS).enumerate() {
            params[4 + i] = effect.as_u32();
        }
        params[12] = comic_levels;
        params
    }

    pub fn set_cursor_grabbed(&self, grabbed: bool) {
        use winit::window::CursorGrabMode;
        self.window.set_cursor_visible(!grabbed);
        if grabbed {
            // Locked = true pointer lock (hides cursor, raw relative motion).
            // Supported on Windows, macOS, Wayland. Not supported on X11.
            // Confined = keeps cursor inside window bounds. Fallback for X11.
            if self.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                let _ = self.window.set_cursor_grab(CursorGrabMode::Confined);
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        }
    }

    pub fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.gpu.resize(size.width, size.height);
        self.recreate_size_dependent_resources();
        self.accumulator.reset();
    }

    pub fn recreate_size_dependent_resources(&mut self) {
        let width = self.gpu.width();
        let height = self.gpu.height();

        let accum_size = (width * height) as u64 * ACCUM_BYTES_PER_PIXEL;
        self.accumulation_buffer =
            buffers::create_empty_storage_buffer(&self.gpu.device, accum_size, "accumulation");

        let (tex, view) = buffers::create_output_texture(&self.gpu.device, width, height, "output");
        self.output_texture = tex;
        self.output_view = view;

        self.compute_bind_group_0 = Self::create_compute_bg0(
            &self.gpu.device,
            &self.compute_bg_layout_0,
            &self.camera_buffer,
            &self.accumulation_buffer,
            &self.output_view,
        );

        self.blit_bind_group = Self::create_blit_bind_group(
            &self.gpu.device,
            &self.blit_bg_layout,
            &self.output_view,
            &self.blit_sampler,
        );

        self.post_bind_group = Self::create_post_bind_group(
            &self.gpu.device,
            &self.post_bg_layout,
            &self.post_params_buffer,
            &self.accumulation_buffer,
            &self.output_view,
        );

        let post_params = Self::build_post_params(
            width,
            height,
            &self.active_effects,
            self.ui_state.oil_radius,
            self.ui_state.comic_levels,
        );
        buffers::update_uniform_buffer(&self.gpu.queue, &self.post_params_buffer, &post_params);
    }

    /// Partition `shapes` into a BVH over finite shapes and a flat list of
    /// infinite-shape indices for linear testing.
    ///
    /// Planes are infinite and would produce degenerate AABBs that corrupt the
    /// BVH tree, so they are excluded from it and tested separately each frame.
    /// Skybox shapes are excluded entirely — they are sampled via `sample_skybox`.
    pub fn build_bvh(shapes: &[Shape]) -> (Bvh, Vec<u32>) {
        let mut finite_to_global: Vec<usize> = Vec::new();
        let mut infinite_indices: Vec<u32> = Vec::new();

        for (i, shape) in shapes.iter().enumerate() {
            match shape.shape_type {
                ShapeType::Plane => infinite_indices.push(i as u32),
                ShapeType::Skybox => {}
                _ => finite_to_global.push(i),
            }
        }

        let finite_aabbs: Vec<_> = finite_to_global
            .iter()
            .map(|&i| shape_aabb(&shapes[i]))
            .collect();
        let mut bvh = Bvh::build(&finite_aabbs);

        // Remap leaf prim_indices from finite-local back to global shape indices.
        for idx in &mut bvh.prim_indices {
            *idx = finite_to_global[*idx as usize] as u32;
        }

        (bvh, infinite_indices)
    }

    fn compute_scene_gpu_data(&self) -> (Vec<GpuShape>, Vec<GpuMaterial>, Vec<u32>, Bvh, Vec<u32>) {
        let (gpu_shapes, gpu_materials, light_indices) =
            Self::build_gpu_data(&self.shapes, &self.tex_path_cache);
        let (bvh, infinite_indices) = Self::build_bvh(&self.shapes);
        (
            gpu_shapes,
            gpu_materials,
            light_indices,
            bvh,
            infinite_indices,
        )
    }

    /// Write updated scene data to existing GPU buffers in-place when they fit.
    /// Falls back to a full rebuild if the BVH grew beyond the current buffer.
    pub fn rebuild_scene_buffers_in_place(&mut self) {
        let (gpu_shapes, gpu_materials, light_indices, bvh, infinite_indices) =
            self.compute_scene_gpu_data();
        self.bvh = bvh;
        self.infinite_indices = infinite_indices;

        let new_node_bytes = std::mem::size_of_val(self.bvh.nodes.as_slice()) as u64;
        if new_node_bytes > self.bvh_node_buffer.size() {
            // BVH grew beyond the current buffer — reallocate so future
            // in-place writes fit without overflow.
            self.rebuild_scene_buffers();
            return;
        }

        buffers::update_storage_buffer(&self.gpu.queue, &self.shape_buffer, &gpu_shapes);
        buffers::update_storage_buffer(&self.gpu.queue, &self.material_buffer, &gpu_materials);
        buffers::update_storage_buffer(&self.gpu.queue, &self.bvh_node_buffer, &self.bvh.nodes);
        buffers::update_storage_buffer(
            &self.gpu.queue,
            &self.bvh_prim_buffer,
            &self.bvh.prim_indices,
        );
        buffers::update_storage_buffer(
            &self.gpu.queue,
            &self.light_index_buffer,
            Self::nonempty_index_buffer(&light_indices),
        );
        buffers::update_storage_buffer(
            &self.gpu.queue,
            &self.infinite_index_buffer,
            Self::nonempty_index_buffer(&self.infinite_indices),
        );
    }

    pub fn rebuild_scene_buffers(&mut self) {
        let (gpu_shapes, gpu_materials, light_indices, bvh, infinite_indices) =
            self.compute_scene_gpu_data();
        self.bvh = bvh;
        self.infinite_indices = infinite_indices;

        let (
            shape_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
            infinite_index_buffer,
        ) = Self::create_geometry_buffers(
            &self.gpu.device,
            &gpu_shapes,
            &gpu_materials,
            &self.bvh,
            &light_indices,
            &self.infinite_indices,
        );
        self.shape_buffer = shape_buffer;
        self.material_buffer = material_buffer;
        self.bvh_node_buffer = bvh_node_buffer;
        self.bvh_prim_buffer = bvh_prim_buffer;
        self.light_index_buffer = light_index_buffer;
        self.infinite_index_buffer = infinite_index_buffer;

        self.compute_bind_group_1 = Self::create_compute_bg1(
            &self.gpu.device,
            &self.compute_bg_layout_1,
            &self.shape_buffer,
            &self.material_buffer,
            &self.bvh_node_buffer,
            &self.bvh_prim_buffer,
            &self.light_index_buffer,
            &self.tex_pixels_buffer,
            &self.tex_infos_buffer,
            &self.infinite_index_buffer,
        );
    }

    pub fn rebuild_scene_buffers_with_textures(&mut self) {
        (self.texture_atlas, self.tex_path_cache) = Self::build_texture_atlas(&self.shapes);

        self.tex_pixels_buffer = buffers::create_storage_buffer(
            &self.gpu.device,
            &self.texture_atlas.pixels,
            "tex_pixels",
            true,
        );
        self.tex_infos_buffer = buffers::create_storage_buffer(
            &self.gpu.device,
            &self.texture_atlas.infos,
            "tex_infos",
            true,
        );

        self.rebuild_scene_buffers();
    }

    fn create_compute_bg0_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute bg0 layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_compute_bg1_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let ro_storage = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute bg1 layout"),
            entries: &[
                ro_storage(0),
                ro_storage(1),
                ro_storage(2),
                ro_storage(3),
                ro_storage(4),
                ro_storage(5),
                ro_storage(6),
                ro_storage(7),
            ],
        })
    }

    fn create_blit_bg_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn create_post_bg_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn create_compute_bg0(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        camera_buf: &wgpu::Buffer,
        accum_buf: &wgpu::Buffer,
        output_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute bg0"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: accum_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
            ],
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_compute_bg1(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        shape_buf: &wgpu::Buffer,
        material_buf: &wgpu::Buffer,
        bvh_node_buf: &wgpu::Buffer,
        bvh_prim_buf: &wgpu::Buffer,
        light_idx_buf: &wgpu::Buffer,
        tex_pixels_buf: &wgpu::Buffer,
        tex_infos_buf: &wgpu::Buffer,
        infinite_idx_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute bg1"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shape_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: material_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bvh_node_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bvh_prim_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: light_idx_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: tex_pixels_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: tex_infos_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: infinite_idx_buf.as_entire_binding(),
                },
            ],
        })
    }

    pub fn create_blit_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        output_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn create_post_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        post_params_buf: &wgpu::Buffer,
        accum_buf: &wgpu::Buffer,
        output_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: post_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: accum_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
            ],
        })
    }
}
