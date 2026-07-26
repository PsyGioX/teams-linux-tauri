mod notifications;
mod theme;
mod tray;
mod shortcuts;
mod dnd;
mod security;
mod session;

use tauri::{Manager, Emitter, Listener, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const TEAMS_BRIDGE_JS: &str = include_str!("../injected/teams-bridge.js");
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
            // on_navigation — вторая линия обороны против фишинговых ссылок
            // из чата: если top-level навигация уходит с доменов Microsoft
            // (см. src/security.rs), она блокируется и вместо этого
            // открывается в системном браузере, где нет моста __TAURI__.
            // Первая линия обороны — capabilities/default.json > remote.urls,
            // который отзывает доступ к Rust-командам ещё до этой проверки.
            let nav_handle = handle.clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(TEAMS_URL.parse()?))
                .title("Microsoft Teams")
                .inner_size(1280.0, 800.0)
                .min_inner_size(860.0, 560.0)
                .user_agent(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15 TeamsLinux/0.1.0",
                )
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
                        // Открываем в обычном браузере пользователя, а не внутри
                        // нашего webview с доступом к Tauri-командам
                        tauri::async_runtime::spawn(async move {
                            let _ = opener_handle.opener().open_url(url_owned.as_str(), None::<&str>);
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
