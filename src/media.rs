//! Автоматическое разрешение runtime-permission-запросов webkit2gtk
//! И явное включение WebRTC/media-стека в настройках WebKit.
//!
//! Почему мало одного permission-request:
//! Разрешение запроса — это только "да, можно открыть камеру/микрофон".
//! Но сам доступ к getUserMedia/RTCPeerConnection управляется отдельными
//! WebKitSettings-флагами (`enable-webrtc`, `enable-media-stream` и т.д.),
//! которые WRY/Tauri **не включают по умолчанию** — по состоянию на 2025-2026
//! официальная поддержка WebRTC в Tauri всё ещё не завершена
//! (см. обсуждение разработчиков: https://github.com/orgs/tauri-apps/discussions/8426,
//! комментарий мейнтейнера FabianLars от 24.12.2023: "It felt like that for
//! a year now... pretty sure we won't see it [officially] soon"). Без явного
//! `set_enable_webrtc(true)` кнопка звонка в Teams просто не получает
//! медиапоток, даже если сам permission разрешён.
//!
//! Официальная документация WebKitGTK прямо говорит: если
//! `WebKitUserMediaPermissionRequest` не обработан явно, он **отклоняется по
//! умолчанию**
//! (https://webkitgtk.org/reference/webkit2gtk/2.14.6/WebKitUserMediaPermissionRequest.html).
//!
//! ВАЖНО (честно о границах этого фикса): по опыту сообщества (см. ссылку
//! выше) полноценный видеопоток в webkit2gtk на **Wayland** мог упираться в
//! ошибки декодирования буфера (`GBM-DRV error`) на некоторых связках
//! GPU-драйвер/GStreamer — это ограничение самого WebKitGTK, а не нашего
//! кода. Если после этого фикса видео в звонке всё ещё не работает, а звук
//! работает — попробуйте временно `GDK_BACKEND=x11 teams-linux` для
//! диагностики (если у вас установлен Xorg/XWayland).

use tauri::webview::WebviewWindow;

/// Подключает автоматическое разрешение permission-запросов и включает
/// WebRTC/media-стек WebKit для главного окна.
#[cfg(target_os = "linux")]
pub fn grant_webkit_media_permissions(window: &WebviewWindow) -> tauri::Result<()> {
    window.with_webview(|webview| {
        use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

        let wk_webview = webview.inner();

        // Явно включаем WebRTC и весь смежный media-стек. Каждая настройка
        // отвечает за свой кусочек: enable_webrtc — сам RTCPeerConnection,
        // enable_media_stream — getUserMedia, enable_mediasource — буферизация
        // потока в звонке, enable_media/enable_media_capabilities — общая
        // поддержка воспроизведения медиа, media_playback_requires_user_gesture
        // (false) — чтобы звук в звонке не требовал отдельного клика по
        // странице перед началом воспроизведения.
        if let Some(settings) = WebViewExt::settings(&wk_webview) {
            settings.set_enable_webrtc(true);
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
            settings.set_enable_media(true);
            settings.set_enable_media_capabilities(true);
            settings.set_enable_encrypted_media(true);
            settings.set_media_playback_requires_user_gesture(false);
            settings.set_media_playback_allows_inline(true);
        } else {
            log::warn!("Не удалось получить WebKitSettings для включения WebRTC");
        }

        // Мы не показываем системный диалог — пользователь уже один раз
        // "доверяет" этому приложению, устанавливая его для доступа к своему
        // рабочему Teams, поэтому дополнительный GTK-диалог "разрешить
        // камеру?" только мешал бы, а не защищал.
        wk_webview.connect_permission_request(|_webview, request| {
            log::info!("WebKit permission-request: автоматически разрешаю (камера/микрофон/экран для звонков Teams)");
            request.allow();
            true
        });
    })
}

#[cfg(not(target_os = "linux"))]
pub fn grant_webkit_media_permissions(_window: &WebviewWindow) -> tauri::Result<()> {
    // Проект целенаправленно ориентирован на Debian/GNOME; на других
    // платформах разрешения камеры/микрофона обрабатываются иначе
    // (системные диалоги macOS/Windows), поэтому здесь ничего не делаем.
    Ok(())
}
