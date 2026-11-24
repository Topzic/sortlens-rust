//! The entry points a phone host calls.
//!
//! These are the only unsafe-adjacent items in the application: a C symbol the platform looks up by
//! name. Each does the minimum — find the app's private directory, build the options — and hands over
//! to [`crate::run`].
#![allow(unsafe_code)]

/// Android: the native activity's entry. `android-activity` calls it on the app's main thread with
/// the handle the window system needs.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    let data_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp/sortlens"));
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    if let Err(error) = crate::run(options, data_dir) {
        eprintln!("sortlens: {error}");
    }
}

/// iOS: called from the app's `main` (`ios/main.m` is one line: `sortlens_ios_main();`). The
/// event loop owns the thread from here and does not return.
#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn sortlens_ios_main() {
    let data_dir = crate::places::ios_data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    if let Err(error) = crate::run(eframe::NativeOptions::default(), data_dir) {
        eprintln!("sortlens: {error}");
    }
}
