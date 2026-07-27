//! Единый allowlist доменов Microsoft, необходимых для работы Teams v2
//! и связанного с ним OAuth-логина (login.microsoftonline.com и т.п.).
//!
//! Используется в src/main.rs::on_navigation как вторая линия обороны
//! (первая — capabilities/default.json > remote.urls, который отключает
//! доступ к window.__TAURI__ на посторонних доменах ещё до того, как
//! успеет сработать эта проверка).
//!
//! Список должен оставаться синхронизированным с capabilities/default.json.
//!
//! ВАЖНО: с 2023 года Microsoft последовательно переводит Teams, Outlook
//! и другие M365-сервисы на новый консолидированный домен верхнего уровня
//! `.microsoft` (`*.cloud.microsoft`, `*.static.microsoft`,
//! `*.usercontent.microsoft`) — см. официальный анонс Microsoft Learn
//! ("Microsoft 365 URLs and IP address ranges" > "Microsoft 365 Unified
//! Domains") и сообщение центра сообщений M365 MC1162275 (сентябрь 2025):
//! Teams уже доступен на `teams.cloud.microsoft` и пользователей начинают
//! автоматически редиректить туда. Без этих доменов в allowlist легитимный
//! редирект самого Teams после логина блокировался как "чужой домен" и
//! открывался во внешнем браузере — именно это чаще всего проявляется как
//! "после авторизации открывается ссылка в браузере".
const ALLOWED_SUFFIXES: &[&str] = &[
    "teams.microsoft.com",
    "microsoftonline.com",
    "office.com",
    "office365.com",
    "live.com",
    "msftauth.net",
    "msauth.net",
    "sharepoint.com",
    "skype.com",
    // CDN-ресурсы, без которых Teams не отрисуется/не загрузит статику
    "microsoft.com",
    "msecnd.net",
    "office.net",
    // Новый консолидированный домен M365 (teams.cloud.microsoft и т.п.),
    // на который Microsoft постепенно переводит все продукты — без этих
    // трёх записей легитимный редирект после логина считался "чужим".
    "cloud.microsoft",
    "static.microsoft",
    "usercontent.microsoft",
    // Официальные линк-шортенеры Microsoft: приглашения на встречи,
    // "Подробнее" в UI, ссылки из писем и т.п. очень часто используют их
    "aka.ms",
    "1drv.ms",
    // Отдельные (не поддомены microsoft.com/office.com) сервисы,
    // на которые могут вести ссылки из Teams
    "microsoftstream.com",
    "yammer.com",
    // Legacy Azure AD / условный доступ (некоторые тенанты всё ещё
    // используют эти домены в цепочке аутентификации)
    "windows.net",
    "windowsazure.com",
];

/// true, если host совпадает с одним из разрешённых доменов Microsoft
/// или является его поддоменом (например, "login.microsoftonline.com"
/// проходит по суффиксу "microsoftonline.com").
pub fn is_allowed_host(host: &str) -> bool {
    let host = host.to_lowercase();
    ALLOWED_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_known_domains() {
        assert!(is_allowed_host("teams.microsoft.com"));
        assert!(is_allowed_host("login.microsoftonline.com"));
        assert!(is_allowed_host("statics.teams.cdn.office.net"));
        assert!(is_allowed_host("teams.cloud.microsoft"));
        assert!(is_allowed_host("aka.ms"));
        assert!(is_allowed_host("web.microsoftstream.com"));
    }

    #[test]
    fn blocks_phishing_lookalikes() {
        assert!(!is_allowed_host("teams-microsoft.com.evil.tld"));
        assert!(!is_allowed_host("microsoftonline.com.attacker.io"));
        assert!(!is_allowed_host("evil.com"));
    }
}
