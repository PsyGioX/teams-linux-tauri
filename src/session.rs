//! Защита локально хранимых данных сессии (куки, localStorage, IndexedDB
//! webview с логином в Teams).
//!
//! Честно: приложение НЕ шифрует эти данные на диске сверх того, что даёт
//! webkit2gtk из коробки — так же, как обычный Firefox/Chrome не шифрует
//! свой профиль без полнодискового шифрования. Реальная защита от кражи
//! сессии при физическом/файловом доступе к машине — LUKS/FDE на уровне ОС,
//! а не приложения (см. README, раздел "Безопасность").
//!
//! Что делает этот модуль:
//!   1. Выставляет права 0700 на каталог данных приложения (defense-in-depth:
//!      без этого каталог может унаследовать более широкие права от umask).
//!   2. Даёт команду `clear_session` (кнопка "Выйти" в трее) — быстрый способ
//!      стереть локальные куки/токены, например перед тем как одолжить
//!      ноутбук или после подозрения на компрометацию машины.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use tauri::{AppHandle, Manager};

/// Вызывается один раз при старте приложения из setup().
pub fn harden_data_dir_permissions(app: &AppHandle) {
    let Ok(dir) = app.path().app_data_dir() else {
        log::warn!("Не удалось определить каталог данных приложения");
        return;
    };
    if let Err(e) = fs::create_dir_all(&dir) {
        log::warn!("Не удалось создать {}: {e}", dir.display());
        return;
    }
    if let Err(e) = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)) {
        log::warn!("Не удалось выставить права 0700 на {}: {e}", dir.display());
    }
}

/// Полностью удаляет локальные данные webview (куки сессии Teams,
/// localStorage, кэш авторизации). После вызова приложение нужно
/// перезапустить — на следующем старте Teams снова покажет форму логина.
#[tauri::command]
pub fn clear_session(app: AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Не удалось очистить сессию: {e}"))?;
    }
    Ok(())
}
