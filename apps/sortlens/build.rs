//! Generate the window icon where `main` can include it.
//!
//! The mark lives in `sortlens-icon` so the committed files and the pixels the window uses are one picture.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../../crates/icon/src/lib.rs");
    println!("cargo:rerun-if-changed=../../crates/icon/src/mark.rgba");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("icon.rgba"), sortlens_icon::rgba()).expect("write icon");
    println!("cargo:rustc-env=SORTLENS_ICON_EDGE={}", sortlens_icon::SIZE);
}
