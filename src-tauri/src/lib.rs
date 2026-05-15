mod config;
mod crypto;
mod note;
mod util;

use tauri::window::Color;
use tauri::Manager;

#[tauri::command]
fn load_config() -> config::Config {
    config::load()
}

#[tauri::command]
fn save_config(cfg: config::Config) {
    config::save(&cfg);
}

#[tauri::command]
fn load_note() -> note::Note {
    note::load()
}

#[tauri::command]
fn save_note(note: note::Note) {
    note::save(&note);
}

#[tauri::command]
fn get_app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Notes".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let cfg = config::load();
            if let Some(window) = app.get_webview_window("main") {
                if config::exists() {
                    let _ = window.set_position(tauri::PhysicalPosition::new(cfg.left, cfg.top));
                    let _ = window.set_size(tauri::PhysicalSize::new(cfg.width, cfg.height));
                } else {
                    let _ = window.set_size(tauri::PhysicalSize::new(cfg.width, cfg.height));
                    let _ = window.center();
                }
                let _ = window.set_always_on_top(cfg.always_on_top);
                #[cfg(windows)]
                let _ = window.set_background_color(Some(Color(30, 30, 30, 255)));
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_name,
            load_config,
            save_config,
            load_note,
            save_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
