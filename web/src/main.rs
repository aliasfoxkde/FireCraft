//! `firecraft-web` — the browser bundle: the same app shell compiled
//! for wasm32 (trunk), plus a postMessage bridge so a host page (the
//! `WebOS`) can open documents, drive registry commands, and pull
//! renders and saves out.
//!
//! The bridge is a surface, not a side door: every document mutation
//! it performs goes through `Editor::apply` → `Session::apply` — the
//! same registry the CLI and desktop shell use — and both exports are
//! pure read paths (`compose::render`, `io::save`).

#[cfg(target_arch = "wasm32")]
mod platform;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!(
            "firecraft-web is the wasm bundle (build it with `trunk build` in web/); \
             the desktop binary is `firecraft` in crate `app`"
        );
    }
}

/// Creates a `JsValue`-backed reply object from `(key, value)` pairs.
#[cfg(target_arch = "wasm32")]
fn reply_object(fields: &[(&str, wasm_bindgen::JsValue)]) -> wasm_bindgen::JsValue {
    let object = js_sys::Object::new();
    for (key, value) in fields {
        let _ = js_sys::Reflect::set(&object, &wasm_bindgen::JsValue::from_str(key), value);
    }
    object.into()
}

/// Posts a reply to the parent frame (same window when top-level).
#[cfg(target_arch = "wasm32")]
fn post_reply(value: &wasm_bindgen::JsValue) {
    use wasm_bindgen::JsValue;
    let parent = web_sys::window().and_then(|window| window.parent().ok().flatten());
    if let Some(parent) = parent {
        // "*" — the WebOS may embed us on any origin; our replies are
        // inert (status flags, rendered pixels, document bytes).
        let _ = parent.post_message(&JsValue::from(value), "*");
    }
}

/// Replies with a JSON-shaped status object.
#[cfg(target_arch = "wasm32")]
fn reply_status(kind: &str, ok: bool, error: Option<&str>) {
    let mut fields = vec![
        ("source", wasm_bindgen::JsValue::from_str("firecraft")),
        ("type", wasm_bindgen::JsValue::from_str(kind)),
        ("ok", wasm_bindgen::JsValue::from_bool(ok)),
    ];
    if let Some(error) = error {
        fields.push(("error", wasm_bindgen::JsValue::from_str(error)));
    }
    post_reply(&reply_object(&fields));
}

/// Best-effort string for a rejected `JsValue` error.
#[cfg(target_arch = "wasm32")]
fn js_error_text(value: wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "unknown error".to_owned())
}

/// Replies with an `ArrayBuffer` payload.
#[cfg(target_arch = "wasm32")]
fn reply_bytes(kind: &str, bytes: &[u8]) {
    let array = js_sys::Uint8Array::from(bytes);
    let buffer = array.buffer();
    post_reply(&reply_object(&[
        ("source", wasm_bindgen::JsValue::from_str("firecraft")),
        ("type", wasm_bindgen::JsValue::from_str(kind)),
        ("bytes", buffer.into()),
    ]));
}

/// Runs `f` against the running shell, if the runner is live. Every
/// bridge call may mutate the shell between frames, so each one wakes
/// the frame loop afterwards — without the repaint, an opened or
/// applied document stayed behind the stale canvas until the next
/// input event.
#[cfg(target_arch = "wasm32")]
fn with_app<T>(
    runner: &std::rc::Rc<eframe::WebRunner>,
    f: impl FnOnce(&mut app::shell::Shell) -> T,
) -> Option<T> {
    runner.app_mut::<app::shell::Shell>().map(|mut shell| {
        let outcome = f(&mut shell);
        shell.request_repaint();
        outcome
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::JsValue;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id("firecraft-canvas")
        .ok_or_else(|| JsValue::from_str("missing #firecraft-canvas element"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let runner = std::rc::Rc::new(eframe::WebRunner::new());

    // Hidden file input: the browser-side open path (async, so it
    // cannot live inside the synchronous Dialogs trait).
    let input = document
        .create_element("input")?
        .dyn_into::<web_sys::HtmlInputElement>()?;
    input.set_type("file");
    // Every format the open door accepts (see `Shell::open_document`):
    // the picker must not hide files the app can open.
    input.set_accept(".fw.png,.png,.jpg,.jpeg,.gif,.bmp,.tif,.tiff,.webp,image/*");
    input.set_id("firecraft-file-input");
    let _ = input.set_attribute(
        "style",
        "position:absolute;width:1px;height:1px;opacity:0;pointer-events:none",
    );
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no document body"))?;
    let _ = body.append_child(&input);

    let input_runner = runner.clone();
    let input_callback = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
        let runner = input_runner.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let picked = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("firecraft-file-input"))
                .and_then(|input| input.dyn_into::<web_sys::HtmlInputElement>().ok());
            let Some(input) = picked else {
                return;
            };
            let Some(file) = input.files().and_then(|files| files.get(0)) else {
                return;
            };
            // Reset first so picking the same file twice re-fires.
            input.set_value("");
            match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                Ok(value) => match value.dyn_into::<js_sys::ArrayBuffer>() {
                    Ok(buffer) => open_bytes(&runner, &js_sys::Uint8Array::new(&buffer).to_vec()),
                    Err(_) => {
                        reply_status("opened", false, Some("picked file is not an ArrayBuffer"))
                    }
                },
                Err(error) => reply_status("opened", false, Some(&js_error_text(error))),
            }
        });
    });
    let _ =
        input.add_event_listener_with_callback("change", input_callback.as_ref().unchecked_ref());
    // The listener must outlive this function — leak it deliberately.
    input_callback.forget();

    // postMessage bridge.
    let message_runner = runner.clone();
    let message_callback = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MessageEvent)>::new(
        move |event: web_sys::MessageEvent| handle_message(&message_runner, event),
    );
    window
        .add_event_listener_with_callback("message", message_callback.as_ref().unchecked_ref())?;
    message_callback.forget();

    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|_cc| {
                Ok(Box::new(app::shell::Shell::with_platform(
                    app::editor::Editor::untitled(),
                    Box::new(platform::WebFs),
                    Box::new(platform::WebDialogs),
                )))
            }),
        )
        .await?;

    // Drop sink: the shell's drag-and-drop handler hands us the
    // dropped-file handle (the read is async on the web); the bytes
    // load through the same `load_document_bytes` door the bridge's
    // openDocument uses, with a repaint so the result is visible
    // immediately.
    let drop_runner = runner.clone();
    app::shell::Shell::set_drop_sink(std::rc::Rc::new(move |file, ctx| {
        let runner = drop_runner.clone();
        let file = file.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match file.bytes_async().await {
                Ok(bytes) => {
                    with_app(&runner, |shell| {
                        let _ = shell.load_document_bytes(&bytes);
                    });
                }
                Err(error) => {
                    with_app(&runner, |shell| {
                        shell.status = format!("open failed: {error}");
                    });
                }
            }
            ctx.request_repaint();
        });
    }));

    reply_status("ready", true, None);
    Ok(())
}

/// Loads document bytes into the shell through the standard read path.
#[cfg(target_arch = "wasm32")]
fn open_bytes(runner: &std::rc::Rc<eframe::WebRunner>, bytes: &[u8]) {
    match with_app(runner, |shell| shell.load_document_bytes(bytes)) {
        Some(Ok(())) => reply_status("opened", true, None),
        Some(Err(error)) => reply_status("opened", false, Some(&error.to_string())),
        None => reply_status("opened", false, Some("app is not running")),
    }
}

/// Dispatches one inbound postMessage. Our own replies (marked
/// `source: "firecraft"`) are ignored; a bare `ArrayBuffer` is
/// shorthand for `openDocument`.
#[cfg(target_arch = "wasm32")]
fn handle_message(runner: &std::rc::Rc<eframe::WebRunner>, event: web_sys::MessageEvent) {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::JsValue;

    let data = event.data();

    if let Ok(source) = js_sys::Reflect::get(&data, &JsValue::from_str("source")) {
        if source.as_string().as_deref() == Some("firecraft") {
            return;
        }
    }

    if let Some(buffer) = data.dyn_ref::<js_sys::ArrayBuffer>() {
        open_bytes(runner, &js_sys::Uint8Array::new(buffer).to_vec());
        return;
    }

    let Some(kind) = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
        .ok()
        .and_then(|value| value.as_string())
    else {
        return;
    };

    match kind.as_str() {
        "openDocument" => {
            let bytes = js_sys::Reflect::get(&data, &JsValue::from_str("bytes"))
                .ok()
                .and_then(|value| value.dyn_into::<js_sys::ArrayBuffer>().ok())
                .map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec());
            match bytes {
                Some(bytes) => open_bytes(runner, &bytes),
                None => reply_status(
                    "opened",
                    false,
                    Some("openDocument: `bytes` must be an ArrayBuffer"),
                ),
            }
        }
        "apply" => {
            let name = js_sys::Reflect::get(&data, &JsValue::from_str("name"))
                .ok()
                .and_then(|value| value.as_string());
            let payload: serde_json::Value =
                js_sys::Reflect::get(&data, &JsValue::from_str("payload"))
                    .map_or(Ok(serde_json::Value::Null), |value| {
                        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
                    })
                    .unwrap_or(serde_json::Value::Null);
            let outcome = with_app(runner, |shell| match name {
                Some(name) => shell
                    .editor
                    .apply(&name, payload)
                    .map_err(|error| error.to_string()),
                None => Err("apply: missing `name`".to_owned()),
            });
            match outcome {
                Some(Ok(())) => reply_status("applied", true, None),
                Some(Err(error)) => reply_status("applied", false, Some(&error)),
                None => reply_status("applied", false, Some("app is not running")),
            }
        }
        "exportPng" => {
            let rendered = with_app(runner, |shell| {
                compose::render(
                    shell.editor.document(),
                    shell.editor.page,
                    shell.editor.frame,
                )
            });
            match rendered {
                Some(Ok(render)) => {
                    let straight = compose::straight_alpha(&render.pixels);
                    let encode = io::Plane::new(
                        u32::from(render.width),
                        u32::from(render.height),
                        &straight,
                    )
                    .and_then(|plane| io::png::encode(&plane));
                    match encode {
                        Ok(png) => reply_bytes("png", &png),
                        Err(error) => reply_status("png", false, Some(&error.to_string())),
                    }
                }
                Some(Err(error)) => reply_status("png", false, Some(&error.to_string())),
                None => reply_status("png", false, Some("app is not running")),
            }
        }
        "exportDocument" => {
            let saved = with_app(runner, |shell| {
                shell.editor.save().map_err(|error| error.to_string())
            });
            match saved {
                Some(Ok(bytes)) => reply_bytes("document", &bytes),
                Some(Err(error)) => reply_status("document", false, Some(&error)),
                None => reply_status("document", false, Some("app is not running")),
            }
        }
        "ping" => reply_status("ready", true, None),
        _ => {}
    }
}
