#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
#[path = "ui/web_controls.rs"]
mod web_controls;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, MouseButton, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::state::State;
#[cfg(not(target_arch = "wasm32"))]
use crate::state::UiAction;
#[cfg(target_arch = "wasm32")]
use web_controls::{WebControls, ensure_web_controls};

pub enum UserEvent {
    #[cfg(target_arch = "wasm32")]
    StateReady(State),
    #[cfg(target_arch = "wasm32")]
    TogglePause,
    #[cfg(target_arch = "wasm32")]
    SetAliveThreshold(f32),
    #[cfg(target_arch = "wasm32")]
    SetCellColor([f32; 3]),
    #[cfg(target_arch = "wasm32")]
    ClearBoard,
    #[cfg(target_arch = "wasm32")]
    LoadImageBytes(Vec<u8>),
    #[cfg(not(target_arch = "wasm32"))]
    OpenImageDialogResult(Option<PathBuf>),
    #[cfg(not(target_arch = "wasm32"))]
    SaveImageDialogResult(Option<PathBuf>),
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    state: Option<State>,
    #[cfg(not(target_arch = "wasm32"))]
    is_dialog_open: bool,
    #[cfg(target_arch = "wasm32")]
    web_controls: Option<WebControls>,
}

impl App {
    pub fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        let proxy = event_loop.create_proxy();
        Self {
            state: None,
            proxy,
            #[cfg(not(target_arch = "wasm32"))]
            is_dialog_open: false,
            #[cfg(target_arch = "wasm32")]
            web_controls: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_image_file_dialog(&mut self) {
        if self.is_dialog_open {
            return;
        }

        self.is_dialog_open = true;
        let proxy = self.proxy.clone();

        // Need to run file dialog in a separate thread to avoid stalling the UI
        std::thread::spawn(move || {
            let selected = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
                .pick_file();

            let _ = proxy.send_event(UserEvent::OpenImageDialogResult(selected));
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_save_image_dialog(&mut self) {
        if self.is_dialog_open {
            return;
        }

        self.is_dialog_open = true;
        let proxy = self.proxy.clone();

        std::thread::spawn(move || {
            let selected = rfd::FileDialog::new()
                .add_filter("PNG Image", &["png"])
                .set_file_name("board.png")
                .save_file();

            let _ = proxy.send_event(UserEvent::SaveImageDialogResult(selected));
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn ensure_web_controls(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        ensure_web_controls(&mut self.web_controls, &self.proxy)
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;

            window_attributes = window_attributes.with_drag_and_drop(false);
        }

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_elt = canvas.unchecked_into();

            window_attributes = window_attributes.with_canvas(Some(html_canvas_elt));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we're not on the web, we can use `pollster` to await
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.ensure_web_controls().unwrap_throw();

            // Run the proxy asynchronously and use it to send the results to the event loop
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                assert!(
                    proxy
                        .send_event(UserEvent::StateReady(
                            State::new(window).await.expect("Failed to create canvas"),
                        ))
                        .is_ok()
                )
            });
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            #[cfg(target_arch = "wasm32")]
            UserEvent::StateReady(mut state) => {
                // `proxy.send_event()` sends initialization event here on wasm.
                #[cfg(target_arch = "wasm32")]
                {
                    state.window.request_redraw();
                    state.resize(
                        state.window.inner_size().width,
                        state.window.inner_size().height,
                    );
                }

                self.state = Some(state);
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::TogglePause => {
                if let Some(state) = &mut self.state {
                    state.toggle_pause();
                }
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::SetAliveThreshold(alive_threshold) => {
                if let Some(state) = &mut self.state {
                    state.set_alive_threshold(alive_threshold);
                }
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::SetCellColor(color) => {
                if let Some(state) = &mut self.state {
                    state.set_cell_color(color);
                }
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::ClearBoard => {
                if let Some(state) = &mut self.state {
                    state.clear_board();
                }
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::LoadImageBytes(bytes) => {
                if let Some(state) = &mut self.state {
                    if let Err(err) = state.load_board_from_image_bytes(&bytes) {
                        log::error!("failed to load uploaded image: {err}");
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            UserEvent::OpenImageDialogResult(path) => {
                self.is_dialog_open = false;

                if let (Some(state), Some(path)) = (&mut self.state, path) {
                    state.load_board_from_image_file(path);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            UserEvent::SaveImageDialogResult(path) => {
                self.is_dialog_open = false;

                if let (Some(state), Some(path)) = (&self.state, path) {
                    state.save_board_to_image_file(path);
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        #[cfg(not(target_arch = "wasm32"))]
        state.handle_window_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::CursorMoved { position, .. } => {
                state.set_cursor_position(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: mouse_state,
                button: MouseButton::Left,
                ..
            } => {
                if mouse_state.is_pressed() {
                    #[cfg(not(target_arch = "wasm32"))]
                    if state.should_capture_mouse() {
                        return;
                    }

                    state.paint_cell_under_cursor();
                }
            }
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) =>
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(action) = state.take_ui_action() {
                            match action {
                                UiAction::OpenImageDialog => self.open_image_file_dialog(),
                                UiAction::SaveImageDialog => self.open_save_image_dialog(),
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("error: {err}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                #[cfg(not(target_arch = "wasm32"))]
                if code == winit::keyboard::KeyCode::KeyU && key_state.is_pressed() {
                    self.open_image_file_dialog();
                    return;
                }

                #[cfg(not(target_arch = "wasm32"))]
                if code == winit::keyboard::KeyCode::KeyB && key_state.is_pressed() {
                    self.open_save_image_dialog();
                    return;
                }

                #[cfg(target_arch = "wasm32")]
                if code == winit::keyboard::KeyCode::KeyU && key_state.is_pressed() {
                    if let Some(document) = wgpu::web_sys::window().and_then(|w| w.document()) {
                        if let Some(element) = document.get_element_by_id("upload-image-input") {
                            if let Ok(input) = element.dyn_into::<web_sys::HtmlInputElement>() {
                                input.click();
                            }
                        }
                    }
                    return;
                }

                state.handle_key(event_loop, code, key_state.is_pressed())
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new(&event_loop);
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
