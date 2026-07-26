//! Следит за `org.gnome.desktop.interface color-scheme` через `gsettings monitor`
//! и инжектит соответствующий CSS во фронтенд Teams, чтобы интерфейс не
//! выглядел чужеродным веб-виджетом среди адвайта-приложений.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Serialize, Clone, PartialEq)]
pub enum SystemTheme {
    Light,
    Dark,
}

/// Разовый синхронный запрос темы (используется при старте окна)
#[tauri::command]
pub fn get_system_theme() -> SystemTheme {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("dark") {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            }
        }
        Err(_) => SystemTheme::Light,
    }
}

/// Фоновый watcher: держит `gsettings monitor` живым и на каждое изменение
/// шлёт событие `teams-linux://theme-changed` в окно с "light" | "dark".
pub fn watch_gnome_theme(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut cmd = match Command::new("gsettings")
            .args(["monitor", "org.gnome.desktop.interface", "color-scheme"])
            .stdout(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("gsettings monitor недоступен ({e}), синхронизация темы отключена");
                return;
            }
        };

        let stdout = match cmd.stdout.take() {
            Some(s) => s,
            None => return,
        };
        let mut lines = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let theme = if line.contains("dark") {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            };
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.emit("teams-linux://theme-changed", &theme);
            }
        }
    });
}
