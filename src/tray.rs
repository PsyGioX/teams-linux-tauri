//! Трей через libayatana-appindicator (GNOME сам по себе трея не имеет,
//! appindicator + расширение AppIndicator Support закрывают это).
//! Меню: Открыть Teams / Не беспокоить / Выход.
//! Счётчик непрочитанных обновляется из фронтенда через событие
//! `teams-linux://set-unread-count` (см. injected/teams-bridge.js).

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Открыть Teams", true, None::<&str>)?;
    let dnd_item = MenuItem::with_id(app, "toggle-dnd", "Не беспокоить", true, None::<&str>)?;
    let logout_item = MenuItem::with_id(
        app,
        "logout",
        "Выйти и очистить сессию",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&open_item, &dnd_item, &separator, &logout_item, &quit_item],
    )?;

    let app_for_events = app.clone();
    let app_for_listener = app.clone();
    let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Microsoft Teams")
        .icon(app.default_window_icon().cloned().unwrap())
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "toggle-dnd" => {
                let _ = app.emit("teams-linux://tray-toggle-dnd", ());
            }
            "logout" => {
                let app = app.clone();
                app.dialog()
                    .message(
                        "Локальные куки и токен авторизации Teams будут удалены. \
                         Понадобится войти заново. Продолжить?",
                    )
                    .title("Выйти и очистить сессию")
                    .kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::OkCancel)
                    .show(move |confirmed| {
                        if !confirmed {
                            return;
                        }
                        match crate::session::clear_session(app.clone()) {
                            Ok(_) => {
                                log::info!("Сессия очищена, завершаю приложение");
                                app.exit(0);
                            }
                            Err(e) => log::error!("Не удалось очистить сессию: {e}"),
                        }
                    });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    // Фронтенд шлёт актуальный счётчик непрочитанных: обновляем tooltip трея
    // и пишем значение в файл, который читает опциональное GNOME Shell
    // расширение teams-indicator@psygiox (extension/ в репозитории).
    app_for_events.listen("teams-linux://set-unread-count", move |event| {
        if let Ok(count) = event.payload().parse::<u32>() {
            let tooltip = if count > 0 {
                format!("Microsoft Teams — {count} непрочитанных")
            } else {
                "Microsoft Teams".to_string()
            };

            if let Some(tray) = app_for_listener.tray_by_id("main-tray") {
                let _ = tray.set_tooltip(Some(tooltip.as_str()));
            }

            write_unread_count_file(count);
        }
    });

    Ok(())
}

/// Пишет число непрочитанных в ~/.local/share/teams-linux/unread-count,
/// который читает опциональное GNOME Shell расширение (см. extension/).
fn write_unread_count_file(count: u32) {
    let Some(mut dir) = dirs::data_local_dir() else {
        return;
    };
    dir.push("teams-linux");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Не удалось создать {}: {e}", dir.display());
        return;
    }
    dir.push("unread-count");
    if let Err(e) = std::fs::write(&dir, count.to_string()) {
        log::warn!("Не удалось записать {}: {e}", dir.display());
    }
}
