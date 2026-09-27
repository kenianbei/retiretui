//! The page's edge: the browser's file dialog for what comes in, and a
//! download link for what goes out.

use std::sync::{Arc, Mutex, PoisonError};

use retiretui_tui::exchange::Exchange;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, Document, HtmlAnchorElement, HtmlInputElement, Url};

/// What the dialog offers: plans, scenarios and SSA statements.
const ACCEPTED: &str = ".toml,.xml";
const TEXT_TYPE: &str = "text/plain";

/// Files the visitor chose, kept until the planner takes them.
#[derive(Debug, Default)]
pub struct PageEdge {
    arrived: Arc<Mutex<Vec<(String, String)>>>,
}

impl Exchange for PageEdge {
    fn ask(&self) {
        if let Err(failure) = open_dialog(Arc::clone(&self.arrived)) {
            warn(&format!("no file dialog: {failure:?}"));
        }
    }

    fn hand_out(&self, name: &str, text: &str) {
        if let Err(failure) = download(name, text) {
            warn(&format!("{name} not downloaded: {failure:?}"));
        }
    }

    fn arrived(&self) -> Vec<(String, String)> {
        let mut arrived = self.arrived.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *arrived)
    }
}

fn document() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("no document"))
}

fn open_dialog(arrived: Arc<Mutex<Vec<(String, String)>>>) -> Result<(), JsValue> {
    let input: HtmlInputElement = document()?.create_element("input")?.dyn_into()?;
    input.set_type("file");
    input.set_multiple(true);
    input.set_accept(ACCEPTED);
    let chosen = input.clone();
    let on_change = Closure::once_into_js(move || take_files(&chosen, &arrived));
    input.set_onchange(Some(on_change.unchecked_ref()));
    input.click();
    Ok(())
}

/// Reads each chosen file, each arriving once the browser has read it.
fn take_files(input: &HtmlInputElement, arrived: &Arc<Mutex<Vec<(String, String)>>>) {
    let Some(files) = input.files() else {
        return;
    };
    for file in (0..files.length()).filter_map(|at| files.item(at)) {
        let arrived = Arc::clone(arrived);
        wasm_bindgen_futures::spawn_local(async move {
            let name = file.name();
            match JsFuture::from(file.text())
                .await
                .map(|text| text.as_string())
            {
                Ok(Some(text)) => arrived
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((name, text)),
                Ok(None) | Err(_) => warn(&format!("{name} could not be read")),
            }
        });
    }
}

fn download(name: &str, text: &str) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&JsValue::from_str(text));
    let kind = BlobPropertyBag::new();
    kind.set_type(TEXT_TYPE);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &kind)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let link: HtmlAnchorElement = document()?.create_element("a")?.dyn_into()?;
    link.set_href(&url);
    link.set_download(name);
    link.click();
    Url::revoke_object_url(&url)
}

fn warn(said: &str) {
    web_sys::console::warn_1(&JsValue::from_str(said));
}
