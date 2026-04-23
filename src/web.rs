use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    crate::ui::run().map_err(|e| JsValue::from_str(&format!("{e}")))
}
