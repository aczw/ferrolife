use std::sync::Arc;

use imgui::{Condition, FontConfig, FontSource};
use imgui_wgpu::{Renderer as ImguiRenderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::window::Window;

use super::State;

pub enum UiAction {
    OpenImageDialog,
    SaveImageDialog,
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
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Default);

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

    let mut is_paused = state.is_paused;
    let mut reset_elapsed = false;
    let mut alive_threshold = state.simulation.alive_threshold();
    let mut update_alive_threshold = false;
    let mut cell_color = [state.cell_color.x, state.cell_color.y, state.cell_color.z];
    let mut update_cell_color = false;
    let mut clear_board = false;
    let mut born_rules = state.simulation.born_rules();
    let mut survive_rules = state.simulation.survive_rules();
    let mut update_born_rules = false;
    let mut update_survive_rules = false;
    {
        let ui = state.ui_layer.imgui.frame();
        ui.window("Controls")
            .position([12.0, 12.0], Condition::FirstUseEver)
            .collapsed(true, Condition::FirstUseEver)
            .movable(false)
            .resizable(false)
            .always_auto_resize(true)
            .build(|| {
                let pause_label = if is_paused { "Resume" } else { "Pause " };
                if ui.button(pause_label) {
                    is_paused = !is_paused;
                    reset_elapsed = true;
                }
                ui.same_line();

                if ui.button("Upload Image") {
                    requested_action = Some(UiAction::OpenImageDialog);
                }
                ui.same_line();

                if ui.button("Save Board") {
                    requested_action = Some(UiAction::SaveImageDialog);
                }
                ui.same_line();

                if ui.button("Clear Board") {
                    clear_board = true;
                }

                if ui.slider("Alive Threshold", 0.0, 1.0, &mut alive_threshold) {
                    update_alive_threshold = true;
                }

                if ui.color_edit3("Cell Color", &mut cell_color) {
                    update_cell_color = true;
                }

                ui.separator();
                ui.text("Born Rules (B):");
                ui.same_line();
                for i in 0..9 {
                    let mask = 1u16 << i;
                    let mut checked = (born_rules & mask) != 0;
                    if ui.checkbox(format!("{}##B{}", i, i), &mut checked) {
                        if checked {
                            born_rules |= mask;
                        } else {
                            born_rules &= !mask;
                        }
                        update_born_rules = true;
                    }
                    ui.same_line();
                }

                ui.new_line();
                ui.text("Survive Rules (S):");
                ui.same_line();
                for i in 0..9 {
                    let mask = 1u16 << i;
                    let mut checked = (survive_rules & mask) != 0;
                    if ui.checkbox(format!("{}##S{}", i, i), &mut checked) {
                        if checked {
                            survive_rules |= mask;
                        } else {
                            survive_rules &= !mask;
                        }
                        update_survive_rules = true;
                    }
                    ui.same_line();
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
    if update_cell_color {
        state.set_cell_color(cell_color);
    }
    if update_born_rules {
        state.set_born_rules(born_rules);
    }
    if update_survive_rules {
        state.set_survive_rules(survive_rules);
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
