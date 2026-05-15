//! System tray icon — shows a colored badge matching the titlebar color.
//!
//! Left-click toggles window visibility. Right-click context menu: Show, Quit.
//! The tray icon generates a 32×32 colored circle based on the titlebar color.

use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

/// Holds the tray icon so we can update it dynamically.
pub struct TrayState<R: Runtime> {
    pub icon: Mutex<Option<TrayIcon<R>>>,
}

impl<R: Runtime> TrayState<R> {
    pub fn new() -> Self {
        Self {
            icon: Mutex::new(None),
        }
    }
}

pub fn build<R: Runtime>(app: &AppHandle<R>, tooltip: &str, initial_color: &str) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_item, &PredefinedMenuItem::separator(app)?, &quit_item],
    )?;

    let icon = generate_colored_icon(initial_color)?; // use persisted or default blue

    let tray = TrayIconBuilder::new()
        .tooltip(tooltip)
        .icon(icon)
        .menu(&menu)
        .on_menu_event(move |handle: &AppHandle<R>, event| {
            let id = event.id().as_ref();
            match id {
                "show" => {
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let handle: &AppHandle<R> = tray.app_handle();
                if let Some(window) = handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    // Store the tray icon in AppState for later updates
    if let Some(state) = app.try_state::<TrayState<R>>() {
        if let Ok(mut guard) = state.icon.lock() {
            *guard = Some(tray);
        }
    }

    Ok(())
}

/// Generate a 32×32 colored circle as a Tauri Image.
pub fn generate_colored_icon(
    hex_color: &str,
) -> Result<Image<'static>, Box<dyn std::error::Error>> {
    let (r, g, b) = parse_hex_color(hex_color).unwrap_or((93, 173, 226));
    let size = 32u32;
    let cx = 16f64;
    let cy = 16f64;
    let radius = 14f64;

    let mut buf = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;

            if dist <= radius {
                // Anti-alias at the edge
                let alpha = if dist > radius - 1.5 {
                    ((radius - dist) / 1.5).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = (alpha * 200.0) as u8;
            } else {
                buf[idx + 3] = 0; // transparent
            }
        }
    }

    let image = Image::new_owned(buf, size, size);
    Ok(image)
}

/// Update the tray icon with a new color.
pub fn update_color<R: Runtime>(app: &AppHandle<R>, hex_color: &str) {
    if let Some(state) = app.try_state::<TrayState<R>>() {
        if let Ok(guard) = state.icon.lock() {
            if let Some(tray) = guard.as_ref() {
                if let Ok(image) = generate_colored_icon(hex_color) {
                    let _ = tray.set_icon(Some(image));
                }
            }
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        u8::from_str_radix(&hex[0..2], 16).ok().and_then(|r| {
            u8::from_str_radix(&hex[2..4], 16)
                .ok()
                .and_then(|g| u8::from_str_radix(&hex[4..6], 16).ok().map(|b| (r, g, b)))
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_hex_color ─────────────────────────────────────────────

    #[test]
    fn test_parse_hex_standard() {
        assert_eq!(parse_hex_color("#ff6b6b"), Some((255, 107, 107)));
    }

    #[test]
    fn test_parse_hex_without_hash() {
        assert_eq!(parse_hex_color("ff6b6b"), Some((255, 107, 107)));
    }

    #[test]
    fn test_parse_hex_black() {
        assert_eq!(parse_hex_color("#000000"), Some((0, 0, 0)));
    }

    #[test]
    fn test_parse_hex_white() {
        assert_eq!(parse_hex_color("#ffffff"), Some((255, 255, 255)));
    }

    #[test]
    fn test_parse_hex_blue() {
        assert_eq!(parse_hex_color("#5dade2"), Some((93, 173, 226)));
    }

    #[test]
    fn test_parse_hex_empty() {
        assert_eq!(parse_hex_color(""), None);
    }

    #[test]
    fn test_parse_hex_short() {
        assert_eq!(parse_hex_color("#fff"), None);
    }

    #[test]
    fn test_parse_hex_invalid_chars() {
        assert_eq!(parse_hex_color("#zzzzzz"), None);
    }

    #[test]
    fn test_parse_hex_partial_invalid() {
        assert_eq!(parse_hex_color("#ff00zz"), None);
    }

    // ── generate_colored_icon ───────────────────────────────────────

    #[test]
    fn test_generate_icon_returns_image() {
        let img = generate_colored_icon("#ff0000").unwrap();
        assert!(!img.rgba().is_empty());
    }

    #[test]
    fn test_generate_icon_default_color() {
        // Empty/invalid hex → unwrap_or → default blue (93, 173, 226)
        let img = generate_colored_icon("").unwrap();
        assert!(!img.rgba().is_empty());
    }

    #[test]
    fn test_generate_icon_size_correct() {
        // 32×32 × 4 bytes = 4096
        let img = generate_colored_icon("#00ff00").unwrap();
        assert_eq!(img.rgba().len(), 4096);
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_generate_icon_red_pixels_exist() {
        let img = generate_colored_icon("#ff0000").unwrap();
        let bytes = img.rgba();
        // At least some pixels should be non-zero (the circle)
        let non_zero = bytes.iter().filter(|&&b| b != 0).count();
        assert!(non_zero > 0, "colored circle should have non-zero pixels");
    }

    #[test]
    fn test_generate_icon_center_pixel_is_red() {
        let img = generate_colored_icon("#ff0000").unwrap();
        let bytes = img.rgba();
        // Center pixel = (16, 16) → index = (16*32+16)*4 = 2064
        let idx = (16 * 32 + 16) * 4;
        assert_eq!(bytes[idx], 255, "R should be 255");     // R
        assert_eq!(bytes[idx + 1], 0, "G should be 0");     // G
        assert_eq!(bytes[idx + 2], 0, "B should be 0");     // B
        assert!(bytes[idx + 3] > 0, "Alpha should be > 0"); // A
    }

    #[test]
    fn test_generate_icon_center_pixel_is_blue() {
        let img = generate_colored_icon("#0000ff").unwrap();
        let bytes = img.rgba();
        let idx = (16 * 32 + 16) * 4;
        assert_eq!(bytes[idx], 0, "R should be 0");
        assert_eq!(bytes[idx + 1], 0, "G should be 0");
        assert_eq!(bytes[idx + 2], 255, "B should be 255");
        assert!(bytes[idx + 3] > 0, "Alpha should be > 0");
    }

    #[test]
    fn test_generate_icon_corner_is_transparent() {
        let img = generate_colored_icon("#ff0000").unwrap();
        let bytes = img.rgba();
        // Corner pixel = (0,0)
        assert_eq!(bytes[3], 0, "corner should be transparent");
        // Corner pixel = (31,0) 
        assert_eq!((31 * 4 + 3) as usize, 127);
        assert_eq!(bytes[127], 0, "corner should be transparent");
        // Corner pixel = (0,31)
        assert_eq!((31 * 32 * 4 + 3) as usize, 3971);
        assert_eq!(bytes[3971], 0, "corner should be transparent");
    }

    // ── Anti-aliasing edge ────────────────────────────────────────

    #[test]
    fn test_generate_icon_edge_pixel_partial_alpha() {
        // Pixels at the edge of the circle (distance ~14) should have
        // alpha between 0 and 200 to verify anti-aliasing.
        let img = generate_colored_icon("#ff0000").unwrap();
        let bytes = img.rgba();
        // edge pixel: (16+13, 16) = (29, 16) → distance 13 → partial alpha
        // At distance 13 from center (radius 14), alpha = (14-13)/1.5 ≈ 0.667
        let idx = ((16 * 32 + 29) * 4) as usize;
        let alpha = bytes[idx + 3];
        assert!(
            alpha > 0 && alpha < 200,
            "edge pixel should have partial alpha, got {}",
            alpha
        );
    }

    #[test]
    fn test_generate_icon_far_outside_transparent() {
        // Pixel far from center should be fully transparent
        let img = generate_colored_icon("#00ff00").unwrap();
        let bytes = img.rgba();
        // (31, 31) — farthest corner from center (16,16)
        let idx = ((31 * 32 + 31) * 4) as usize;
        assert_eq!(bytes[idx + 3], 0, "far corner should be transparent");
    }
}
