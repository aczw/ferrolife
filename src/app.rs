#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::state::State;
#[cfg(not(target_arch = "wasm32"))]
use crate::state::UiAction;

pub enum UserEvent {
    #[cfg(target_arch = "wasm32")]
    StateReady(State),
    #[cfg(target_arch = "wasm32")]
    TogglePause,
    #[cfg(target_arch = "wasm32")]
    SetAliveThreshold(f32),
    #[cfg(not(target_arch = "wasm32"))]
    FileDialogResult(Option<PathBuf>),
}

#[cfg(target_arch = "wasm32")]
struct WebControls {
    _container: web_sys::Element,
    _pause_click: Closure<dyn FnMut(web_sys::Event)>,
    _alive_threshold_input: Closure<dyn FnMut(web_sys::Event)>,
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    state: Option<State>,
    #[cfg(not(target_arch = "wasm32"))]
    is_file_dialog_open: bool,
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
            is_file_dialog_open: false,
            #[cfg(target_arch = "wasm32")]
            web_controls: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_file_dialog(&mut self) {
        if self.is_file_dialog_open {
            return;
        }

        self.is_file_dialog_open = true;
        let proxy = self.proxy.clone();

        // Need to run file dialog in a separate thread to avoid stalling the UI
        std::thread::spawn(move || {
            let selected = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
                .pick_file();

            let _ = proxy.send_event(UserEvent::FileDialogResult(selected));
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn ensure_web_controls(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        if self.web_controls.is_some() {
            return Ok(());
        }

        let window = wgpu::web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let body = document.body().unwrap_throw();

        let container = document.create_element("div")?;
        container.set_attribute(
            "style",
            "position:fixed;top:12px;left:12px;z-index:50;display:flex;gap:8px;align-items:center;padding:8px 10px;border-radius:8px;background:rgba(24,24,24,0.68);backdrop-filter:blur(2px);",
        )?;

        let pause_button = document.create_element("button")?;
        pause_button.set_text_content(Some("Pause/Resume"));
        pause_button.set_attribute(
            "style",
            "border:0;border-radius:6px;padding:6px 10px;cursor:pointer;background:#f0f0f0;color:#1f1f1f;font:600 13px sans-serif;",
        )?;

        let alive_label = document.create_element("span")?;
        alive_label.set_text_content(Some("Threshold"));
        alive_label.set_attribute("style", "color:#f5f5f5;font:500 12px sans-serif;")?;

        let alive_slider: web_sys::HtmlInputElement =
            document.create_element("input")?.dyn_into()?;
        alive_slider.set_type("range");
        alive_slider.set_min("0.0");
        alive_slider.set_max("1.0");
        alive_slider.set_step("0.01");
        alive_slider.set_value("0.30");
        alive_slider.set_attribute("style", "width:140px;")?;

        container.append_child(&pause_button)?;
        container.append_child(&alive_label)?;
        container.append_child(&alive_slider)?;
        body.append_child(&container)?;

        let pause_proxy = self.proxy.clone();
        let pause_click = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let _ = pause_proxy.send_event(UserEvent::TogglePause);
        }) as Box<dyn FnMut(_)>);
        pause_button
            .add_event_listener_with_callback("click", pause_click.as_ref().unchecked_ref())?;

        let threshold_proxy = self.proxy.clone();
        let alive_slider_for_input = alive_slider.clone();
        let alive_threshold_input = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let parsed = alive_slider_for_input
                .value()
                .parse::<f32>()
                .unwrap_or(0.30)
                .clamp(0.0, 1.0);
            let _ = threshold_proxy.send_event(UserEvent::SetAliveThreshold(parsed));
        }) as Box<dyn FnMut(_)>);
        alive_slider.add_event_listener_with_callback(
            "input",
            alive_threshold_input.as_ref().unchecked_ref(),
        )?;

        self.web_controls = Some(WebControls {
            _container: container,
            _pause_click: pause_click,
            _alive_threshold_input: alive_threshold_input,
        });

        Ok(())
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

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
            #[cfg(not(target_arch = "wasm32"))]
            UserEvent::FileDialogResult(path) => {
                self.is_file_dialog_open = false;

                if let (Some(state), Some(path)) = (&mut self.state, path) {
                    state.handle_dropped_file(path);
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
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::DroppedFile(path) => state.handle_dropped_file(path),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) =>
                    {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(action) = state.take_ui_action() {
                            match action {
                                UiAction::OpenImageDialog => self.open_file_dialog(),
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
                    self.open_file_dialog();
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
