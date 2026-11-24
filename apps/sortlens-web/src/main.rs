//! Sortlens: the browser client's entry point.
//!
//! Built for `wasm32` by `cargo xtask web` and served by the paired host itself
//! (`--web dist`, the `/app` route): the shell is the page, the library lives in memory, and
//! everything else is the server's (`docs/ARCHITECTURE.md` §10). A native build of this
//! crate is not the app — it says so and stops.

#![forbid(unsafe_code)]

/// The native binary is not the browser client; say what is, and why there is no window.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("sortlens-web is the browser client (SL-J02): a wasm page, not a native app.");
    eprintln!("Build it with `cargo xtask web` and serve it with `sortlens-server --web apps/sortlens-web/dist`.");
    std::process::exit(2);
}

/// The wasm binary needs a `main` symbol for `cargo` to build it; the entry that actually
/// runs is [`start`], invoked by the generated glue when the page loads.
#[cfg(target_arch = "wasm32")]
fn main() {}

/// The page boots itself: find the canvas the HTML provides, install the panic hook so a
/// crash is a console error the user can report, and run the shell.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
fn start() -> Result<(), wasm_bindgen::JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!("sortlens-web panicked: {info}")));
    }));
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = sortlens_web::web::boot().await {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "sortlens-web could not start: {error:?}"
            )));
        }
    });
    Ok(())
}
