#[cfg(not(target_arch = "wasm32"))]
#[path = "ui/imgui_controls.rs"]
mod imgui_controls;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;

use cgmath::Vector4;
use web_time::Instant;
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::{
    camera::{self, Camera},
    cells::Cells,
    simulation::{GRID_HEIGHT, GRID_WIDTH, Simulation},
};

#[cfg(not(target_arch = "wasm32"))]
pub use imgui_controls::UiAction;
#[cfg(not(target_arch = "wasm32"))]
use imgui_controls::{UiLayer, create_ui_layer, record_ui};

fn clamp_surface_size(width: u32, height: u32, max_dimension: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width.max(1), height.max(1));
    }

    let scale = (max_dimension as f32 / width as f32)
        .min(max_dimension as f32 / height as f32)
        .min(1.0);

    let clamped_width = ((width as f32 * scale).round() as u32).clamp(1, max_dimension);
    let clamped_height = ((height as f32 * scale).round() as u32).clamp(1, max_dimension);

    (clamped_width, clamped_height)
}

pub struct Surface {
    handle: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    is_configured: bool,
}

pub struct State {
    pub window: Arc<Window>,

    surface: Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera: Camera,
    camera_unif: camera::Uniform,
    camera_unif_buf: wgpu::Buffer,
    camera_bg: wgpu::BindGroup,
    camera_controller: camera::Controller,

    simulation: Simulation,
    cells: Cells,

    previous_instant: Instant,
    elapsed: f32,
    is_paused: bool,
    live_cell_color: Vector4<f32>,
    cursor_pos: Option<(f32, f32)>,

    #[cfg(not(target_arch = "wasm32"))]
    ui_layer: UiLayer,
    #[cfg(not(target_arch = "wasm32"))]
    pending_ui_action: Option<UiAction>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let window_size = window.inner_size();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let max_surface_dimension = device.limits().max_texture_dimension_2d;
        let (surface_width, surface_height) =
            clamp_surface_size(window_size.width, window_size.height, max_surface_dimension);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: surface_width,
            height: surface_height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        let camera = Camera::new(30.0, surface_width as f32 / surface_height as f32);
        let mut camera_unif = camera::Uniform::new();
        camera_unif.update_view_proj(&camera);
        let camera_unif_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera-unif-buf"),
            contents: bytemuck::cast_slice(&[camera_unif]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_unif_buf.as_entire_binding(),
            }],
        });

        let camera_controller = camera::Controller::new(0.2, 0.05);

        let simulation = Simulation::new(&device);
        let cells = Cells::new(&device, &surface_config, &camera_bgl);

        #[cfg(not(target_arch = "wasm32"))]
        let ui_layer = create_ui_layer(&window, &device, &queue, surface_format);

        Ok(Self {
            window,

            surface: Surface {
                handle: surface,
                config: surface_config,
                is_configured: false,
            },
            device,
            queue,

            camera,
            camera_unif,
            camera_unif_buf,
            camera_bg,
            camera_controller,

            simulation,
            cells,

            previous_instant: Instant::now(),
            elapsed: 0.0,
            is_paused: false,
            live_cell_color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            cursor_pos: None,

            #[cfg(not(target_arch = "wasm32"))]
            ui_layer,
            #[cfg(not(target_arch = "wasm32"))]
            pending_ui_action: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let surface = &mut self.surface;
            let max_surface_dimension = self.device.limits().max_texture_dimension_2d;
            let (surface_width, surface_height) =
                clamp_surface_size(width, height, max_surface_dimension);

            surface.config.width = surface_width;
            surface.config.height = surface_height;
            surface.handle.configure(&self.device, &surface.config);
            surface.is_configured = true;

            self.cells.resize(&self.device, &surface.config);
            self.camera
                .update_aspect_ratio(surface_width as f32 / surface_height as f32);

            self.update();
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.surface.is_configured {
            return Ok(());
        }

        let now = Instant::now();
        let delta_time = now - self.previous_instant;
        self.previous_instant = now;
        if !self.is_paused {
            self.elapsed += delta_time.as_secs_f32();
        }

        let (output, should_reconfigure_after_present) =
            match self.surface.handle.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(surface_texture) => (surface_texture, false),
                wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                    // Reconfigure only after presenting and dropping this frame output
                    (surface_texture, true)
                }
                wgpu::CurrentSurfaceTexture::Timeout
                | wgpu::CurrentSurfaceTexture::Occluded
                | wgpu::CurrentSurfaceTexture::Validation => {
                    return Ok(()); // Skip this frame
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.surface
                        .handle
                        .configure(&self.device, &self.surface.config);
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    // We would have to recreate the devices and all resources created,
                    // but we'll just bail now
                    anyhow::bail!("Lost device");
                }
            };

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("surface-view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command-encoder"),
            });

        {
            let num_instances = self.simulation.num_instances() as u32;

            let instance_buf_to_use = if !self.is_paused && self.elapsed >= 0.5 {
                self.elapsed = 0.0;
                self.simulation.record(&mut encoder, &self.device)
            } else {
                self.simulation.current_instance_buf_to_use()
            };

            self.cells.record(
                &mut encoder,
                &surface_view,
                &self.camera_bg,
                instance_buf_to_use,
                num_instances,
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pending_ui_action = self.record_ui(&mut encoder, &surface_view, delta_time)?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        if should_reconfigure_after_present {
            self.surface
                .handle
                .configure(&self.device, &self.surface.config);
        }

        Ok(())
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else if code == KeyCode::Space && is_pressed {
            self.toggle_pause();
        } else if self.camera_controller.handle_key(code, is_pressed) {
            self.update();
        }
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        self.elapsed = 0.0;
    }

    pub fn set_alive_threshold(&mut self, alive_threshold: f32) {
        self.simulation
            .set_alive_threshold(&self.queue, alive_threshold);
    }

    pub fn set_live_cell_color(&mut self, color: [f32; 3]) {
        self.live_cell_color = Vector4::new(
            color[0].clamp(0.0, 1.0),
            color[1].clamp(0.0, 1.0),
            color[2].clamp(0.0, 1.0),
            1.0,
        );
    }

    pub fn clear_board(&mut self) {
        self.simulation.clear_board(&self.queue);
        self.elapsed = 0.0;
    }

    pub fn set_cursor_position(&mut self, x: f32, y: f32) {
        self.cursor_pos = Some((x, y));
    }

    pub fn paint_cell_under_cursor(&mut self) {
        let Some((screen_x, screen_y)) = self.cursor_pos else {
            return;
        };

        let window_size = self.window.inner_size();
        if let Some((world_x, world_y)) = self.camera.world_pos_from_screen(
            screen_x,
            screen_y,
            window_size.width as f32,
            window_size.height as f32,
        ) {
            let half_grid_width = (GRID_WIDTH as f32 - 1.0) * 0.5;
            let half_grid_height = (GRID_HEIGHT as f32 - 1.0) * 0.5;

            let grid_x = (world_x + half_grid_width + 0.5).floor() as i32;
            let grid_y = (world_y + half_grid_height + 0.5).floor() as i32;

            if grid_x >= 0
                && grid_x < GRID_WIDTH as i32
                && grid_y >= 0
                && grid_y < GRID_HEIGHT as i32
            {
                self.simulation.set_cell_color(
                    &self.queue,
                    grid_x as u32,
                    grid_y as u32,
                    self.live_cell_color,
                );
            }
        }
    }

    pub fn load_board_from_image_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let image = image::load_from_memory(bytes)?;
        let rgba_image = image.to_rgba8();
        let (image_width, image_height) = rgba_image.dimensions();

        self.simulation.set_state_from_rgba_image(
            &self.queue,
            image_width,
            image_height,
            rgba_image.as_raw(),
        )?;

        self.is_paused = true;
        self.elapsed = 0.0;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        let wrapped_event = winit::event::Event::<()>::WindowEvent {
            window_id: self.window.id(),
            event: event.clone(),
        };

        self.ui_layer.platform.handle_event(
            self.ui_layer.imgui.io_mut(),
            &self.window,
            &wrapped_event,
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn should_capture_mouse(&self) -> bool {
        self.ui_layer.imgui.io().want_capture_mouse
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn take_ui_action(&mut self) -> Option<UiAction> {
        self.pending_ui_action.take()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_board_from_image_path(&mut self, path: &Path) -> anyhow::Result<()> {
        let bytes = std::fs::read(path)?;
        self.load_board_from_image_bytes(&bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_board_from_image_file(&mut self, path: std::path::PathBuf) {
        if let Err(err) = self.load_board_from_image_path(&path) {
            log::error!("failed to load image {}: {err}", path.display());
        }
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_unif.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_unif_buf,
            0,
            bytemuck::cast_slice(&[self.camera_unif]),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn record_ui(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        delta_time: web_time::Duration,
    ) -> anyhow::Result<Option<UiAction>> {
        record_ui(self, encoder, surface_view, delta_time)
    }
}
