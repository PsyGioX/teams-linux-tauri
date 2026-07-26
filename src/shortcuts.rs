//! Глобальные системные хоткеи для управления звонком Teams даже когда
//! окно не в фокусе: Super+Shift+M — mute/unmute, Super+Shift+O — камера.
//! Хоткей вызывает JS-инъекцию, которая кликает по нативной кнопке Teams
//! по её aria-label (см. injected/teams-bridge.js::toggleMute/toggleCamera).

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn register_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mute_shortcut: Shortcut = "Super+Shift+M".parse().expect("некорректный шорткат mute");
    let camera_shortcut: Shortcut = "Super+Shift+O".parse().expect("некорректный шорткат camera");

    let handle = app.clone();
    app.global_shortcut().on_shortcut(mute_shortcut, move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = handle.emit("teams-linux://hotkey-toggle-mute", ());
        }
    })?;

    let handle_cam = app.clone();
    app.global_shortcut().on_shortcut(camera_shortcut, move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = handle_cam.emit("teams-linux://hotkey-toggle-camera", ());
        }
    })?;

    Ok(())
}
