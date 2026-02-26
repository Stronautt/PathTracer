use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use bytemuck::Zeroable;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::accel::aabb::figure_aabb;
use crate::accel::bvh::Bvh;
use crate::camera::camera::Camera;
use crate::camera::controller::CameraController;
use crate::gpu::buffers;
use crate::gpu::context::GpuContext;
use crate::input::handler;
use crate::render::accumulator::Accumulator;
use crate::render::post_process::PostEffect;
use crate::scene::figure::{Figure, GpuFigure};
use crate::scene::material::GpuMaterial;
use crate::scene::scene::Scene;
use crate::shaders::composer::ShaderComposer;
use crate::ui;

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

struct AppState {
    window: Arc<Window>,
    gpu: GpuContext,
    scene: Scene,
    figures: Vec<Figure>,
    compute_pipeline: wgpu::ComputePipeline,
    blit_pipeline: wgpu::RenderPipeline,
    post_process_pipeline: wgpu::ComputePipeline,
    camera_buffer: wgpu::Buffer,
    accumulation_buffer: wgpu::Buffer,
    figure_buffer: wgpu::Buffer,
    material_buffer: wgpu::Buffer,
    bvh_node_buffer: wgpu::Buffer,
    bvh_prim_buffer: wgpu::Buffer,
    light_index_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    compute_bind_group_0: wgpu::BindGroup,
    compute_bind_group_1: wgpu::BindGroup,
    blit_bind_group: wgpu::BindGroup,
    post_bind_group: wgpu::BindGroup,
    compute_bg_layout_0: wgpu::BindGroupLayout,
    compute_bg_layout_1: wgpu::BindGroupLayout,
    blit_bg_layout: wgpu::BindGroupLayout,
    post_bg_layout: wgpu::BindGroupLayout,
    post_params_buffer: wgpu::Buffer,
    bvh: Bvh,
    camera: Camera,
    controller: CameraController,
    accumulator: Accumulator,
    drag_figure: Option<usize>,
    drag_depth: f32,
    drag_offset: glam::Vec3,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    ui_state: ui::UiState,
    last_frame: Instant,
    frame_index: u32,
    active_effect: PostEffect,
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
        let Some(state) = &mut self.state else {
            return;
        };

        // Always pass keyboard events to camera controller before egui,
        // so camera keys (WASD, M, numpad, etc.) work regardless of UI state.
        let is_keyboard = matches!(&event, WindowEvent::KeyboardInput { .. });
        if is_keyboard {
            if let WindowEvent::KeyboardInput {
                event: key_event, ..
            } = &event
                && key_event.physical_key == PhysicalKey::Code(KeyCode::Escape)
            {
                if state.controller.mouse_look_key {
                    state.controller.mouse_look_key = false;
                    state.set_cursor_grabbed(false);
                    state.controller.clear_mouse_delta();
                } else if state.controller.mouse_captured {
                    state.controller.mouse_captured = false;
                } else {
                    event_loop.exit();
                }
                return;
            }
            let was_mouse_look = state.controller.mouse_look_key;
            handler::handle_window_event(&event, &mut state.controller);
            if state.controller.mouse_look_key != was_mouse_look {
                state.set_cursor_grabbed(state.controller.mouse_look_key);
                state.controller.clear_mouse_delta();
            }
        }

        if let WindowEvent::CursorMoved { position, .. } = &event {
            state
                .controller
                .handle_cursor_moved(position.x as f32, position.y as f32);
        }

        // In mouse look mode, don't forward events to egui so the UI doesn't
        // react to mouse movement / clicks and doesn't override cursor visibility.
        let in_mouse_look = state.controller.mouse_look_key || state.controller.mouse_captured;
        if !in_mouse_look {
            let egui_response = state.egui_state.on_window_event(&state.window, &event);
            if egui_response.consumed {
                return;
            }
        }

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                state.handle_resize(*size);
            }
            WindowEvent::RedrawRequested => {
                state.update_and_render();
                state.window.request_redraw();
                return;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if !state.controller.mouse_captured && !state.controller.mouse_look_key => {
                if let Some((cx, cy)) = state.controller.last_cursor_pos() {
                    let (origin, dir) = crate::picking::picking_ray(
                        &state.camera,
                        cx,
                        cy,
                        state.gpu.width(),
                        state.gpu.height(),
                    );
                    if let Some((idx, t, hit_point)) =
                        crate::picking::pick(origin, dir, &state.bvh, &state.figures)
                    {
                        state.ui_state.selected_figure = Some(idx);
                        let fig_pos = glam::Vec3::from(state.figures[idx].position);
                        state.drag_figure = Some(idx);
                        state.drag_depth = t;
                        state.drag_offset = hit_point - fig_pos;
                    } else {
                        state.ui_state.selected_figure = None;
                        state.drag_figure = None;
                    }
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                if state.drag_figure.is_some() {
                    state.drag_figure = None;
                    state.rebuild_scene_buffers();
                    state.accumulator.reset();
                }
            }
            WindowEvent::CursorMoved { position, .. } if state.drag_figure.is_some() => {
                if let Some(idx) = state.drag_figure {
                    let (origin, dir) = crate::picking::picking_ray(
                        &state.camera,
                        position.x as f32,
                        position.y as f32,
                        state.gpu.width(),
                        state.gpu.height(),
                    );
                    let new_pos = origin + dir * state.drag_depth - state.drag_offset;
                    state.figures[idx].position = new_pos.into();
                    state.update_single_figure(idx);
                    state.accumulator.reset();
                }
            }
            // Focus loss: release cursor and clear all input state so camera
            // doesn't keep moving when the user alt-tabs away.
            WindowEvent::Focused(false) => {
                state.controller.mouse_look_key = false;
                state.controller.mouse_captured = false;
                state.controller.clear_movement();
                state.controller.clear_mouse_delta();
                state.set_cursor_grabbed(false);
            }
            _ => {}
        }

        if !is_keyboard {
            let was_captured = state.controller.mouse_captured;
            handler::handle_window_event(&event, &mut state.controller);
            if state.controller.mouse_captured != was_captured {
                state.controller.clear_mouse_delta();
            }
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

impl AppState {
    fn new(event_loop: &ActiveEventLoop, scene_path: &Option<String>) -> Result<Self> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("PathTracer")
                    .with_inner_size(PhysicalSize::new(1280u32, 720u32)),
            )?,
        );

        let gpu = GpuContext::new(window.clone())?;
        let width = gpu.width();
        let height = gpu.height();

        let scene = if let Some(path) = scene_path {
            crate::scene::loader::load_scene(Path::new(path))?
        } else {
            crate::scene::loader::load_scene(Path::new("resources/scenes/demo.json"))
                .unwrap_or_else(|_| Scene::empty())
        };

        let camera = Camera::new(
            scene.camera.position.into(),
            scene.camera.rotation,
            scene.camera.fov,
            scene.camera.exposure,
        );

        let figures = scene.figures.clone();
        let (gpu_figures, gpu_materials, light_indices) = Self::build_gpu_data(&figures);

        let aabbs: Vec<_> = figures.iter().map(figure_aabb).collect();
        let bvh = Bvh::build(&aabbs);

        let composer = ShaderComposer::from_directory(&ShaderComposer::shader_dir())?;
        let trace_source = composer.compose("path_trace")?;
        let blit_source = composer.compose("blit")?;
        let post_source = composer.compose("post_process")?;

        let gpu_camera = camera.to_gpu(width, height, 0, 0);
        let camera_buffer = buffers::create_uniform_buffer(&gpu.device, &gpu_camera, "camera");

        let accum_size = (width * height) as u64 * 16; // vec4<f32> per pixel
        let accumulation_buffer =
            buffers::create_empty_storage_buffer(&gpu.device, accum_size, "accumulation");

        let (output_texture, output_view) =
            buffers::create_output_texture(&gpu.device, width, height, "output");

        let (figure_buffer, material_buffer, bvh_node_buffer, bvh_prim_buffer, light_index_buffer) =
            Self::create_scene_buffers(
                &gpu.device,
                &gpu_figures,
                &gpu_materials,
                &bvh,
                &light_indices,
            );

        let post_params: [u32; 4] = [PostEffect::None.as_u32(), width, height, 0];
        let post_params_buffer =
            buffers::create_uniform_buffer(&gpu.device, &post_params, "post_params");

        let compute_bg_layout_0 =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                });

        let compute_bg_layout_1 =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("compute bg1 layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let blit_bg_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                });

        let post_bg_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                });

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
            &figure_buffer,
            &material_buffer,
            &bvh_node_buffer,
            &bvh_prim_buffer,
            &light_index_buffer,
        );

        let blit_bind_group =
            Self::create_blit_bind_group(&gpu.device, &blit_bg_layout, &output_view);
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

        let ui_state = ui::UiState {
            exposure: camera.exposure,
            ..Default::default()
        };

        Ok(Self {
            window,
            gpu,
            scene,
            figures,
            compute_pipeline,
            blit_pipeline,
            post_process_pipeline,
            camera_buffer,
            accumulation_buffer,
            figure_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
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
            bvh,
            camera,
            controller: CameraController::new(),
            accumulator: Accumulator::default(),
            drag_figure: None,
            drag_depth: 0.0,
            drag_offset: glam::Vec3::ZERO,
            egui_ctx,
            egui_state,
            egui_renderer,
            ui_state,
            last_frame: Instant::now(),
            frame_index: 0,
            active_effect: PostEffect::None,
        })
    }

    fn build_gpu_data(figures: &[Figure]) -> (Vec<GpuFigure>, Vec<GpuMaterial>, Vec<u32>) {
        let mut gpu_figures = Vec::with_capacity(figures.len());
        let mut gpu_materials = Vec::with_capacity(figures.len());
        let mut light_indices = Vec::new();

        for (i, fig) in figures.iter().enumerate() {
            let mat_idx = gpu_materials.len() as u32;
            gpu_materials.push(GpuMaterial::from(&fig.material));
            gpu_figures.push(GpuFigure::from_figure(fig, mat_idx));

            if fig.material.is_emissive() {
                light_indices.push(i as u32);
            }
        }

        (gpu_figures, gpu_materials, light_indices)
    }

    fn create_compute_bg0(
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

    fn create_compute_bg1(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        figure_buf: &wgpu::Buffer,
        material_buf: &wgpu::Buffer,
        bvh_node_buf: &wgpu::Buffer,
        bvh_prim_buf: &wgpu::Buffer,
        light_idx_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute bg1"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: figure_buf.as_entire_binding(),
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
            ],
        })
    }

    fn create_scene_buffers(
        device: &wgpu::Device,
        gpu_figures: &[GpuFigure],
        gpu_materials: &[GpuMaterial],
        bvh: &Bvh,
        light_indices: &[u32],
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
    ) {
        let figure_buffer = if gpu_figures.is_empty() {
            buffers::create_storage_buffer(device, &[GpuFigure::zeroed()], "figures", true)
        } else {
            buffers::create_storage_buffer(device, gpu_figures, "figures", true)
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

        // wgpu requires non-empty buffers; sentinel 0xFFFFFFFF is an invalid light index
        let light_data: &[u32] = if light_indices.is_empty() {
            &[0xFFFFFFFF]
        } else {
            light_indices
        };
        let light_index_buffer =
            buffers::create_storage_buffer(device, light_data, "light_indices", true);

        (
            figure_buffer,
            material_buffer,
            bvh_node_buffer,
            bvh_prim_buffer,
            light_index_buffer,
        )
    }

    fn create_blit_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        output_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
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
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    fn create_post_bind_group(
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

    fn set_cursor_grabbed(&self, grabbed: bool) {
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

    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.gpu.resize(size.width, size.height);
        self.recreate_size_dependent_resources();
        self.accumulator.reset();
    }

    fn recreate_size_dependent_resources(&mut self) {
        let width = self.gpu.width();
        let height = self.gpu.height();

        let accum_size = (width * height) as u64 * 16;
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

        self.blit_bind_group =
            Self::create_blit_bind_group(&self.gpu.device, &self.blit_bg_layout, &self.output_view);

        self.post_bind_group = Self::create_post_bind_group(
            &self.gpu.device,
            &self.post_bg_layout,
            &self.post_params_buffer,
            &self.accumulation_buffer,
            &self.output_view,
        );

        let post_params: [u32; 4] = [self.active_effect.as_u32(), width, height, 0];
        buffers::update_uniform_buffer(&self.gpu.queue, &self.post_params_buffer, &post_params);
    }

    fn add_figure(&mut self, fig_type: crate::scene::figure::FigureType) {
        use crate::scene::figure::FigureType;
        let mut fig = Figure {
            figure_type: fig_type,
            position: self.camera.position.into(),
            normal: [0.0, 1.0, 0.0],
            radius: 1.0,
            radius2: 0.3,
            height: 2.0,
            rotation: [0.0, 0.0, 0.0],
            v0: [0.0, 0.0, 0.0],
            v1: [1.0, 0.0, 0.0],
            v2: [0.0, 1.0, 0.0],
            material: crate::scene::material::Material::default(),
        };
        let (_, _, forward) = self.camera.basis_vectors();
        let spawn_pos = self.camera.position + forward * 5.0;
        fig.position = spawn_pos.into();

        if fig_type == FigureType::Plane {
            fig.position = [0.0, 0.0, 0.0];
        }

        self.figures.push(fig);
        self.rebuild_scene_buffers();
        self.accumulator.reset();
        log::info!("Added {:?} figure", fig_type);
    }

    fn delete_figure(&mut self, idx: usize) {
        if idx < self.figures.len() {
            self.figures.remove(idx);
            if let Some(sel) = self.ui_state.selected_figure {
                if sel == idx {
                    self.ui_state.selected_figure = None;
                } else if sel > idx {
                    self.ui_state.selected_figure = Some(sel - 1);
                }
            }
            self.rebuild_scene_buffers();
            self.accumulator.reset();
            log::info!("Deleted figure at index {}", idx);
        }
    }

    fn save_scene(&self) {
        let scene = Scene {
            camera: crate::scene::scene::CameraConfig {
                position: self.camera.position.into(),
                rotation: [self.camera.pitch, self.camera.yaw, 0.0],
                fov: self.camera.fov,
                exposure: self.camera.exposure,
            },
            figures: self.figures.clone(),
            models: vec![],
        };
        if let Err(e) = crate::scene::exporter::save_scene(&scene, Path::new("scene_saved.json")) {
            log::error!("Failed to save scene: {e:#}");
        }
    }

    fn rebuild_scene_buffers(&mut self) {
        let (gpu_figures, gpu_materials, light_indices) = Self::build_gpu_data(&self.figures);

        let aabbs: Vec<_> = self.figures.iter().map(figure_aabb).collect();
        self.bvh = Bvh::build(&aabbs);

        (
            self.figure_buffer,
            self.material_buffer,
            self.bvh_node_buffer,
            self.bvh_prim_buffer,
            self.light_index_buffer,
        ) = Self::create_scene_buffers(
            &self.gpu.device,
            &gpu_figures,
            &gpu_materials,
            &self.bvh,
            &light_indices,
        );

        self.compute_bind_group_1 = Self::create_compute_bg1(
            &self.gpu.device,
            &self.compute_bg_layout_1,
            &self.figure_buffer,
            &self.material_buffer,
            &self.bvh_node_buffer,
            &self.bvh_prim_buffer,
            &self.light_index_buffer,
        );
    }

    /// Fast path for dragging: update only the moved figure in the existing GPU buffer
    /// without rebuilding the BVH or reallocating buffers. The full rebuild happens on release.
    fn update_single_figure(&self, idx: usize) {
        let gpu_fig = GpuFigure::from_figure(&self.figures[idx], idx as u32);
        let offset = (idx * std::mem::size_of::<GpuFigure>()) as u64;
        self.gpu
            .queue
            .write_buffer(&self.figure_buffer, offset, bytemuck::bytes_of(&gpu_fig));
    }

    fn update_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.ui_state.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
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
            ui_actions = ui::draw_ui(ctx, &mut self.ui_state, &mut self.figures);
        });

        if let Some(exp) = ui_actions.exposure_changed {
            self.camera.exposure = exp;
            self.accumulator.reset();
        }
        if let Some(effect) = ui_actions.effect_changed {
            self.active_effect = effect;
            let params: [u32; 4] = [effect.as_u32(), self.gpu.width(), self.gpu.height(), 0];
            buffers::update_uniform_buffer(&self.gpu.queue, &self.post_params_buffer, &params);
        }
        if let Some(fig_type) = ui_actions.figure_to_add {
            self.add_figure(fig_type);
        }
        if let Some(idx) = ui_actions.figure_to_delete {
            self.delete_figure(idx);
        }
        if ui_actions.scene_dirty {
            self.rebuild_scene_buffers();
            self.accumulator.reset();
        }
        if ui_actions.save_requested {
            self.save_scene();
        }
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

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if !self.ui_state.paused {
            // Clear on GPU to avoid a 14MB+ CPU allocation per reset.
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

            if self.active_effect != PostEffect::None {
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
}
