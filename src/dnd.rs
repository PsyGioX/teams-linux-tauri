//! Синхронизирует системный Do Not Disturb GNOME
//! (`org.gnome.desktop.notifications show-banners`) со статусом
//! присутствия в Teams: когда система в DND, фронтенд может
//! автоматически проставлять статус "Не беспокоить".

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Serialize, Clone)]
pub struct DndState {
    pub enabled: bool,
}

#[tauri::command]
pub fn get_dnd_state() -> DndState {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.notifications", "show-banners"])
        .output();

    let enabled = match output {
        // show-banners = false означает, что баннеры отключены -> системный DND включён
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "false",
        Err(_) => false,
    };

    DndState { enabled }
}

pub fn watch_dnd(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut cmd = match Command::new("gsettings")
            .args(["monitor", "org.gnome.desktop.notifications", "show-banners"])
            .stdout(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Не удалось запустить мониторинг DND: {e}");
                return;
            }
        };

        let stdout = match cmd.stdout.take() {
            Some(s) => s,
            None => return,
        };
        let mut lines = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let enabled = line.trim_end().ends_with("false");
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.emit("teams-linux://dnd-changed", DndState { enabled });
            }
        }
    });
}
