//! Мост между Web Notification API (перехваченным в injected/teams-bridge.js)
//! и нативными уведомлениями GNOME через org.freedesktop.Notifications.
//!
//! Даёт кнопки действий (Ответить / Прочитано) прямо в Notification Center,
//! чего нет у обычных webkit-уведомлений.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use zbus::{proxy, zvariant::Value, Connection};

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;
}

static CONV_MAP: LazyLock<Mutex<HashMap<u32, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
pub struct NativeNotificationPayload {
    pub title: String,
    pub body: String,
    /// Идентификатор чата/канала Teams, чтобы знать куда вернуться по клику
    pub conversation_id: Option<String>,
    /// Показывать ли кнопку "Ответить"
    pub allow_reply: bool,
}

#[derive(Serialize, Clone)]
pub struct NotificationActionEvent {
    pub notification_id: u32,
    pub action: String,
    pub conversation_id: Option<String>,
}

/// Вызывается из JS через `invoke('send_native_notification', {...})`
#[tauri::command]
pub async fn send_native_notification(payload: NativeNotificationPayload) -> Result<u32, String> {
    let connection = Connection::session()
        .await
        .map_err(|e| format!("Не удалось подключиться к сессионной DBus: {e}"))?;
    let proxy = NotificationsProxy::new(&connection)
        .await
        .map_err(|e| e.to_string())?;

    let mut actions: Vec<&str> = vec!["default", "Открыть"];
    if payload.allow_reply {
        actions.push("reply");
        actions.push("Ответить");
    }
    actions.push("mark-read");
    actions.push("Прочитано");

    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
    hints.insert("category", Value::from("im.received"));
    hints.insert("desktop-entry", Value::from("teams-linux"));

    let id = proxy
        .notify(
            "Microsoft Teams",
            0,
            "dev.psygiox.teams-linux",
            &payload.title,
            &payload.body,
            &actions,
            hints,
            5000,
        )
        .await
        .map_err(|e| e.to_string())?;

    if let Some(cid) = payload.conversation_id {
        CONV_MAP.lock().unwrap().insert(id, cid);
    }

    Ok(id)
}

/// Заглушка-хук: фронтенд может отчитаться, что действие обработано (для логов/дебага)
#[tauri::command]
pub fn notify_action_result(_result: String) {}

/// Фоновая задача: слушает ActionInvoked и пересылает событие во фронтенд Teams
/// (JS слушает `teams-linux://notification-action`), чтобы например открыть
/// нужный чат или отметить сообщение прочитанным без фокуса окна.
pub fn start_dbus_listener(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let connection = match Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("DBus session недоступна: {e}");
                return;
            }
        };
        let proxy = match NotificationsProxy::new(&connection).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Не удалось создать DBus proxy уведомлений: {e}");
                return;
            }
        };

        let mut stream = match proxy.receive_action_invoked().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("Не удалось подписаться на ActionInvoked: {e}");
                return;
            }
        };

        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                let cid = CONV_MAP.lock().unwrap().get(&args.id).cloned();
                let event = NotificationActionEvent {
                    notification_id: args.id,
                    action: args.action_key.clone(),
                    conversation_id: cid,
                };
                let _ = handle.emit("teams-linux://notification-action", event);
            }
        }
    });
}
