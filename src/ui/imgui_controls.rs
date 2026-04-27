use std::sync::Arc;

use imgui::{Condition, FontConfig, FontSource};
use imgui_wgpu::{Renderer as ImguiRenderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::window::Window;

use super::State;

pub enum UiAction {
    OpenImageDialog,
}

pub(super) struct UiLayer {
    pub(super) imgui: imgui::Context,
    pub(super) platform: WinitPlatform,
    pub(super) renderer: ImguiRenderer,
}

pub(super) fn create_ui_layer(
    window: &Arc<Window>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
) -> UiLayer {
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: 14.0,
            pixel_snap_h: true,
            oversample_h: 1,
            ..FontConfig::default()
        }),
    }]);

    let mut platform = WinitPlatform::new(&mut imgui);
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);

    let renderer_config = RendererConfig {
        texture_format: surface_format,
        ..RendererConfig::default()
    };
    let renderer = ImguiRenderer::new(&mut imgui, device, queue, renderer_config);

    UiLayer {
        imgui,
        platform,
        renderer,
    }
}

pub(super) fn record_ui(
    state: &mut State,
    encoder: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    delta_time: web_time::Duration,
) -> anyhow::Result<Option<UiAction>> {
    let mut requested_action = None;

    state.ui_layer.imgui.io_mut().update_delta_time(delta_time);
    state
        .ui_layer
        .platform
        .prepare_frame(state.ui_layer.imgui.io_mut(), &state.window)?;

    // Keep ImGui clip/scissor conversion aligned with the actual swapchain size.
    let window_size = state.window.inner_size();
    if window_size.width > 0 && window_size.height > 0 {
        let io = state.ui_layer.imgui.io_mut();
        io.display_size = [window_size.width as f32, window_size.height as f32];
        io.display_framebuffer_scale = [
            state.surface.config.width as f32 / window_size.width as f32,
            state.surface.config.height as f32 / window_size.height as f32,
        ];
    }

    let mut is_paused = state.is_paused;
    let mut reset_elapsed = false;
    let mut alive_threshold = state.simulation.alive_threshold();
    let mut update_alive_threshold = false;
    let mut live_cell_color = [
        state.live_cell_color.x,
        state.live_cell_color.y,
        state.live_cell_color.z,
    ];
    let mut update_live_cell_color = false;
    let mut clear_board = false;
    {
        let ui = state.ui_layer.imgui.frame();
        ui.window("Controls")
            .position([12.0, 12.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .movable(false)
            .resizable(false)
            .always_auto_resize(true)
            .build(|| {
                let pause_label = if is_paused { "Resume" } else { "Pause" };
                if ui.button(pause_label) {
                    is_paused = !is_paused;
                    reset_elapsed = true;
                }

                if ui.button("Upload Image") {
                    requested_action = Some(UiAction::OpenImageDialog);
                }

                if ui.button("Clear Board") {
                    clear_board = true;
                }

                if ui.slider("Alive Threshold", 0.0, 1.0, &mut alive_threshold) {
                    update_alive_threshold = true;
                }

                if ui.color_edit3("Live Cell Color", &mut live_cell_color) {
                    update_live_cell_color = true;
                }
            });

        state.ui_layer.platform.prepare_render(ui, &state.window);
    }

    if reset_elapsed {
        state.elapsed = 0.0;
    }
    if update_alive_threshold {
        state.set_alive_threshold(alive_threshold);
    }
    if update_live_cell_color {
        state.set_live_cell_color(live_cell_color);
    }
    if clear_board {
        state.clear_board();
    }
    state.is_paused = is_paused;

    let draw_data = state.ui_layer.imgui.render();
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("imgui-render-pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: surface_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    });

    state
        .ui_layer
        .renderer
        .render(draw_data, &state.queue, &state.device, &mut render_pass)?;

    Ok(requested_action)
}
