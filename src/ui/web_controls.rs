use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::Uint8Array;
use winit::event_loop::EventLoopProxy;

use super::UserEvent;

pub(super) struct WebControls {
    _container: web_sys::Element,
    _pause_click: Closure<dyn FnMut(web_sys::Event)>,
    _upload_image_click: Closure<dyn FnMut(web_sys::Event)>,
    _upload_input_change: Closure<dyn FnMut(web_sys::Event)>,
    _upload_input: web_sys::HtmlInputElement,
    _alive_threshold_input: Closure<dyn FnMut(web_sys::Event)>,
    _cell_color_input: Closure<dyn FnMut(web_sys::Event)>,
    _clear_board_click: Closure<dyn FnMut(web_sys::Event)>,
    _born_rule_closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
    _survive_rule_closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
}

fn parse_html_hex_color(value: &str) -> Option<[f32; 3]> {
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }

    let r = u8::from_str_radix(&value[1..3], 16).ok()?;
    let g = u8::from_str_radix(&value[3..5], 16).ok()?;
    let b = u8::from_str_radix(&value[5..7], 16).ok()?;

    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

pub(super) fn ensure_web_controls(
    controls: &mut Option<WebControls>,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), wasm_bindgen::JsValue> {
    if controls.is_some() {
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

    let upload_image_button = document.create_element("button")?;
    upload_image_button.set_text_content(Some("Upload Image"));
    upload_image_button.set_attribute(
        "style",
        "border:0;border-radius:6px;padding:6px 10px;cursor:pointer;background:#f0f0f0;color:#1f1f1f;font:600 13px sans-serif;",
    )?;

    let upload_input: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
    upload_input.set_type("file");
    upload_input.set_accept("image/*");
    upload_input.set_id("upload-image-input");
    upload_input.set_attribute("style", "display:none;")?;

    let alive_label = document.create_element("span")?;
    alive_label.set_text_content(Some("Threshold"));
    alive_label.set_attribute("style", "color:#f5f5f5;font:500 12px sans-serif;")?;

    let alive_slider: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
    alive_slider.set_type("range");
    alive_slider.set_min("0.0");
    alive_slider.set_max("1.0");
    alive_slider.set_step("0.01");
    alive_slider.set_value("0.30");
    alive_slider.set_attribute("style", "width:140px;")?;

    let cell_color_label = document.create_element("span")?;
    cell_color_label.set_text_content(Some("Cell Color"));
    cell_color_label.set_attribute("style", "color:#f5f5f5;font:500 12px sans-serif;")?;

    let cell_color_input: web_sys::HtmlInputElement =
        document.create_element("input")?.dyn_into()?;
    cell_color_input.set_type("color");
    cell_color_input.set_value("#ffffff");
    cell_color_input.set_attribute("style", "width:40px;height:28px;padding:0;border:0;")?;

    let clear_board_button = document.create_element("button")?;
    clear_board_button.set_text_content(Some("Clear Board"));
    clear_board_button.set_attribute(
        "style",
        "border:0;border-radius:6px;padding:6px 10px;cursor:pointer;background:#f0f0f0;color:#1f1f1f;font:600 13px sans-serif;",
    )?;

    let born_rules = Rc::new(Cell::new(0b0_0000_1000u16));
    let survive_rules = Rc::new(Cell::new(0b0_0000_1100u16));

    // Born and Survive rules sections (in a separate container to allow wrapping)
    let rules_container = document.create_element("div")?;
    rules_container.set_attribute(
        "style",
        "position:fixed;top:90px;left:12px;z-index:50;display:flex;flex-direction:column;gap:8px;padding:8px 10px;border-radius:8px;background:rgba(24,24,24,0.68);backdrop-filter:blur(2px);max-width:600px;",
    )?;

    let born_label = document.create_element("div")?;
    born_label.set_text_content(Some("Born Rules (B):"));
    born_label.set_attribute(
        "style",
        "color:#f5f5f5;font:500 12px sans-serif;margin-bottom:4px;",
    )?;
    rules_container.append_child(&born_label)?;

    let born_row = document.create_element("div")?;
    born_row.set_attribute(
        "style",
        "display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px;",
    )?;
    rules_container.append_child(&born_row)?;

    let survive_label = document.create_element("div")?;
    survive_label.set_text_content(Some("Survive Rules (S):"));
    survive_label.set_attribute(
        "style",
        "color:#f5f5f5;font:500 12px sans-serif;margin-bottom:4px;margin-top:8px;",
    )?;
    rules_container.append_child(&survive_label)?;

    let survive_row = document.create_element("div")?;
    survive_row.set_attribute("style", "display:flex;gap:8px;flex-wrap:wrap;")?;
    rules_container.append_child(&survive_row)?;

    body.append_child(&rules_container)?;

    container.append_child(&pause_button)?;
    container.append_child(&upload_image_button)?;
    container.append_child(&alive_label)?;
    container.append_child(&alive_slider)?;
    container.append_child(&cell_color_label)?;
    container.append_child(&cell_color_input)?;
    container.append_child(&clear_board_button)?;
    body.append_child(&container)?;
    body.append_child(&upload_input)?;

    let pause_proxy = proxy.clone();
    let pause_click = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let _ = pause_proxy.send_event(UserEvent::TogglePause);
    }) as Box<dyn FnMut(_)>);
    pause_button.add_event_listener_with_callback("click", pause_click.as_ref().unchecked_ref())?;

    let upload_input_for_click = upload_input.clone();
    let upload_image_click = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        upload_input_for_click.click();
    }) as Box<dyn FnMut(_)>);
    upload_image_button
        .add_event_listener_with_callback("click", upload_image_click.as_ref().unchecked_ref())?;

    let upload_proxy = proxy.clone();
    let upload_input_for_change = upload_input.clone();
    let upload_input_change = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let Some(files) = upload_input_for_change.files() else {
            return;
        };
        let Some(file) = files.get(0) else {
            return;
        };

        let proxy = upload_proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => {
                    let bytes = Uint8Array::new(&buffer).to_vec();
                    let _ = proxy.send_event(UserEvent::LoadImageBytes(bytes));
                }
                Err(err) => {
                    log::error!("failed reading uploaded image bytes: {err:?}");
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    upload_input
        .add_event_listener_with_callback("change", upload_input_change.as_ref().unchecked_ref())?;

    let threshold_proxy = proxy.clone();
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

    let color_proxy = proxy.clone();
    let cell_color_input_for_event = cell_color_input.clone();
    let cell_color_input_for_callback = cell_color_input.clone();
    let cell_color_input = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        if let Some(color) = parse_html_hex_color(&cell_color_input_for_callback.value()) {
            let _ = color_proxy.send_event(UserEvent::SetCellColor(color));
        }
    }) as Box<dyn FnMut(_)>);
    cell_color_input_for_event
        .add_event_listener_with_callback("input", cell_color_input.as_ref().unchecked_ref())?;

    let clear_proxy = proxy.clone();
    let clear_board_click = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let _ = clear_proxy.send_event(UserEvent::ClearBoard);
    }) as Box<dyn FnMut(_)>);
    clear_board_button
        .add_event_listener_with_callback("click", clear_board_click.as_ref().unchecked_ref())?;

    // Create born rule checkboxes
    let mut born_rule_closures = Vec::new();
    for i in 0..9 {
        let checkbox: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
        checkbox.set_type("checkbox");
        checkbox.set_id(&format!("born-rule-{}", i));
        checkbox.set_attribute("style", "cursor:pointer;width:18px;height:18px;")?;
        checkbox.set_checked((born_rules.get() & (1u16 << i)) != 0);

        let checkbox_label = document.create_element("label")?;
        checkbox_label.set_text_content(Some(&i.to_string()));
        checkbox_label.set_attribute("for", &format!("born-rule-{}", i))?;
        checkbox_label.set_attribute("style", "color:#f5f5f5;font:500 11px sans-serif;cursor:pointer;margin-left:3px;margin-right:6px;")?;

        let rule_proxy = proxy.clone();
        let born_rules = born_rules.clone();
        let checkbox_for_event = checkbox.clone();
        let index = i;
        let born_rule_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let bit_mask = 1u16 << index;
            let mut rules = born_rules.get();
            if checkbox_for_event.checked() {
                rules |= bit_mask;
            } else {
                rules &= !bit_mask;
            }
            born_rules.set(rules);
            let _ = rule_proxy.send_event(UserEvent::SetBornRules(rules));
        }) as Box<dyn FnMut(_)>);
        checkbox.add_event_listener_with_callback(
            "change",
            born_rule_closure.as_ref().unchecked_ref(),
        )?;

        born_row.append_child(&checkbox)?;
        born_row.append_child(&checkbox_label)?;
        born_rule_closures.push(born_rule_closure);
    }

    // Create survive rule checkboxes
    let mut survive_rule_closures = Vec::new();
    for i in 0..9 {
        let checkbox: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
        checkbox.set_type("checkbox");
        checkbox.set_id(&format!("survive-rule-{}", i));
        checkbox.set_attribute("style", "cursor:pointer;width:18px;height:18px;")?;
        checkbox.set_checked((survive_rules.get() & (1u16 << i)) != 0);

        let checkbox_label = document.create_element("label")?;
        checkbox_label.set_text_content(Some(&i.to_string()));
        checkbox_label.set_attribute("for", &format!("survive-rule-{}", i))?;
        checkbox_label.set_attribute("style", "color:#f5f5f5;font:500 11px sans-serif;cursor:pointer;margin-left:3px;margin-right:6px;")?;

        let rule_proxy = proxy.clone();
        let survive_rules = survive_rules.clone();
        let checkbox_for_event = checkbox.clone();
        let index = i;
        let survive_rule_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let bit_mask = 1u16 << index;
            let mut rules = survive_rules.get();
            if checkbox_for_event.checked() {
                rules |= bit_mask;
            } else {
                rules &= !bit_mask;
            }
            survive_rules.set(rules);
            let _ = rule_proxy.send_event(UserEvent::SetSurviveRules(rules));
        }) as Box<dyn FnMut(_)>);
        checkbox.add_event_listener_with_callback(
            "change",
            survive_rule_closure.as_ref().unchecked_ref(),
        )?;

        survive_row.append_child(&checkbox)?;
        survive_row.append_child(&checkbox_label)?;
        survive_rule_closures.push(survive_rule_closure);
    }

    *controls = Some(WebControls {
        _container: container,
        _pause_click: pause_click,
        _upload_image_click: upload_image_click,
        _upload_input_change: upload_input_change,
        _upload_input: upload_input,
        _alive_threshold_input: alive_threshold_input,
        _cell_color_input: cell_color_input,
        _clear_board_click: clear_board_click,
        _born_rule_closures: born_rule_closures,
        _survive_rule_closures: survive_rule_closures,
    });

    Ok(())
}
