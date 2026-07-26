//! Единый allowlist доменов Microsoft, необходимых для работы Teams v2
//! и связанного с ним OAuth-логина (login.microsoftonline.com и т.п.).
//!
//! Используется в src/main.rs::on_navigation как вторая линия обороны
//! (первая — capabilities/default.json > remote.urls, который отключает
//! доступ к window.__TAURI__ на посторонних доменах ещё до того, как
//! успеет сработать эта проверка).
//!
//! Список должен оставаться синхронизированным с capabilities/default.json.

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
    }

    #[test]
    fn blocks_phishing_lookalikes() {
        assert!(!is_allowed_host("teams-microsoft.com.evil.tld"));
        assert!(!is_allowed_host("microsoftonline.com.attacker.io"));
        assert!(!is_allowed_host("evil.com"));
    }
}
