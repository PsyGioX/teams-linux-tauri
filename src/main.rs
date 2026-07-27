mod notifications;
mod theme;
mod tray;
mod shortcuts;
mod dnd;
mod security;
mod session;
mod media;

use tauri::{Manager, Emitter, Listener, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

const TEAMS_BRIDGE_JS: &str = include_str!("../injected/teams-bridge.js");
const UA_DATA_SHIM_JS: &str = include_str!("../injected/user-agent-data-shim.js");
const TEAMS_URL: &str = "https://teams.microsoft.com/v2/";

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Второй запуск: просто поднимаем существующее окно
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            notifications::send_native_notification,
            notifications::notify_action_result,
            theme::get_system_theme,
            dnd::get_dnd_state,
            session::clear_session,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Права 0700 на каталог с куками/localStorage сессии Teams
            session::harden_data_dir_permissions(&handle);

            // Главное окно: грузим Teams v2 напрямую и на каждой загрузке
            // (включая SPA-переходы внутри Teams) инжектим наш JS-мост.
            //
            // User-Agent намеренно выдаёт себя за Chrome/Chromium, а не за
            // реальный движок (webkit2gtk технически ближе к Safari).
            // Причина: по официальной документации Microsoft
            // (learn.microsoft.com/microsoftteams/unsupported-browsers,
            // support.microsoft.com "Join a Teams meeting on an unsupported
            // browser") Teams явно урезает звонки и создание встреч в
            // браузерах, определённых как Safari/Firefox — калькинг-фичи
            // гарантированно доступны только в Chromium-based браузерах
            // (Edge, Chrome). С исходным UA (Safari/605.1.15) Teams
            // определял webkit2gtk как Safari и молча скрывал кнопки
            // звонка/создания встречи — это и есть основная причина второй
            // проблемы из багрепорта.
            //
            // on_navigation — вторая линия обороны против фишинговых ссылок
            // из чата: если top-level навигация уходит с доменов Microsoft
            // (см. src/security.rs), она блокируется и вместо этого
            // открывается в системном браузере, где нет моста __TAURI__.
            // Первая линия обороны — capabilities/default.json > remote.urls,
            // который отзывает доступ к Rust-командам ещё до этой проверки.
            let nav_handle = handle.clone();
            let main_window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(TEAMS_URL.parse()?))
                .title("Microsoft Teams")
                .inner_size(1280.0, 800.0)
                .min_inner_size(860.0, 560.0)
                .user_agent(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
                )
                // Выполняется на этапе document-start, ДО любого скрипта
                // самой страницы Teams — в отличие от on_page_load+eval
                // ниже, который срабатывает уже после начала загрузки и не
                // гарантирует, что успеет раньше первого же скрипта Teams.
                // Именно поэтому шим User-Agent Client Hints вынесен сюда,
                // а не в teams-bridge.js (см. комментарий в самом файле).
                .initialization_script(UA_DATA_SHIM_JS)
                .on_navigation(move |url| {
                    let Some(host) = url.host_str() else {
                        log::warn!("Заблокирована навигация без хоста: {url}");
                        return false;
                    };
                    if security::is_allowed_host(host) {
                        true
                    } else {
                        log::warn!("Заблокирована навигация на посторонний домен: {url}");
                        let opener_handle = nav_handle.clone();
                        let url_owned = url.clone();
                        // Не открываем сразу — показываем модалку с самим
                        // адресом и спрашиваем подтверждение. Без этого
                        // пользователь не видит, куда его вообще уводит
                        // ссылка из чата, и ссылка на подозрительный домен
                        // тихо открывалась бы в системном браузере.
                        opener_handle
                            .dialog()
                            .message(format!(
                                "Ссылка ведёт на сторонний домен:\n\n{url_owned}\n\n\
                                 Открыть её в браузере по умолчанию? (ОК — открыть,\n\
                                 Отмена — остаться в Teams)"
                            ))
                            .title("Переход по внешней ссылке")
                            .kind(MessageDialogKind::Warning)
                            .buttons(MessageDialogButtons::OkCancel)
                            .show(move |confirmed| {
                                if !confirmed {
                                    log::info!("Пользователь отменил переход на {url_owned}");
                                    return;
                                }
                                let _ = opener_handle
                                    .opener()
                                    .open_url(url_owned.as_str(), None::<&str>);
                            });
                        false
                    }
                })
                .on_page_load(|window, _payload| {
                    if let Err(e) = window.eval(TEAMS_BRIDGE_JS) {
                        log::warn!("Не удалось инжектировать teams-bridge.js: {e}");
                    }
                })
                .build()?;

            // Автоматически разрешаем WebKit permission-request (камера,
            // микрофон, экран) — без этого звонок/видео-встреча не
            // запускается вообще, см. подробный комментарий в src/media.rs.
            if let Err(e) = media::grant_webkit_media_permissions(&main_window) {
                log::warn!("Не удалось подключить обработчик WebKit permission-request: {e}");
            }

            // Закрытие окна крестиком сворачивает в трей, а НЕ завершает
            // процесс. Раньше окно реально уничтожалось (или процесс
            // завершался), из-за чего webkit2gtk не успевал синхронно
            // сбросить на диск куки/localStorage сессии Teams — это и
            // проявлялось как "разлогинивает при закрытии через крестик".
            // Реальный выход — только через пункт трея "Выход"
            // (см. src/tray.rs::graceful_quit), который сначала прячет
            // окно и даёт GLib-циклу время на запись данных перед
            // завершением процесса.
            main_window.on_window_event({
                let window_for_close = main_window.clone();
                move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                }
            });

            // Трей с меню Open/Mute/Quit и счётчиком непрочитанных
            tray::build_tray(&handle)?;

            // Глобальные хоткеи mute/камера (Super+Shift+M / Super+Shift+O)
            shortcuts::register_shortcuts(&handle)?;

            // Слушаем изменения темы GNOME (gsettings monitor) и шлём событие во фронтенд
            theme::watch_gnome_theme(handle.clone());

            // Слушаем DBus-сигналы Notify/ActionInvoked для ответа Teams на действия из уведомлений
            notifications::start_dbus_listener(handle.clone());

            // Синхронизация Do Not Disturb GNOME <-> статус в Teams
            dnd::watch_dnd(handle.clone());

            // Ловим deep link msteams:// и teams:// и передаём URL в веб-контент
            let dl_handle = handle.clone();
            app.listen("deep-link://new-url", move |event| {
                let _ = dl_handle.emit("teams-linux://deep-link", event.payload().to_string());
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска приложения Teams Linux");
}
