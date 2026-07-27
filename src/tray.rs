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
    let about_item = MenuItem::with_id(app, "about", "О программе", true, None::<&str>)?;
    let devtools_item = MenuItem::with_id(
        app,
        "devtools",
        "Открыть инструменты разработчика",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let separator3 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_item,
            &dnd_item,
            &separator,
            &logout_item,
            &about_item,
            &separator2,
            &devtools_item,
            &separator3,
            &quit_item,
        ],
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
                                graceful_quit(&app);
                            }
                            Err(e) => log::error!("Не удалось очистить сессию: {e}"),
                        }
                    });
            }
            "about" => {
                app.dialog()
                    .message(format!(
                        "Teams Linux {}\n\n\
                         Неофициальный клиент Microsoft Teams для Debian/GNOME.\n\
                         Не аффилирован с Microsoft.\n\n\
                         Автор: PsyGioX (PsyGioX)\n\
                         GitHub: github.com/PsyGioX/teams-linux-tauri",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .title("О программе")
                    .kind(MessageDialogKind::Info)
                    .show(|_| {});
            }
            "devtools" => {
                // Открывает встроенный WebKit Inspector поверх окна Teams.
                // Полезно, когда звонок/встреча не запускаются: вкладка
                // Console покажет реальную ошибку JS (например, Teams
                // калькинг-стек calling-pluginless-*.js пишет туда, почему
                // именно он отказался стартовать звонок), а не догадки.
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    window.open_devtools();
                }
            }
            "quit" => {
                graceful_quit(app);
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

/// Плавный выход вместо мгновенного `app.exit(0)`.
///
/// Раньше выход через трей (и закрытие окна крестиком, до соответствующего
/// фикса в src/main.rs) мгновенно убивал процесс. webkit2gtk пишет куки и
/// localStorage сессии Teams на диск асинхронно, через idle-колбэки GLib
/// main loop — при резком `exit()` этот loop не успевает докрутиться, и
/// часть сессии терялась, из-за чего при следующем запуске Teams требовал
/// повторный логин ("разлогинивает при закрытии"). Здесь мы сначала скрываем
/// окно (это уже само по себе даёт webkit2gtk сигнал на паузу/сохранение),
/// затем ждём короткую паузу, давая GLib-циклу обработать отложенные задачи
/// записи на диск, и только потом завершаем процесс.
fn graceful_quit(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        app.exit(0);
    });
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
