//! Aviso de versão nova: lógica pura (JSON do GitHub, versões, quando verificar) e a busca HTTP.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use ray_core::Settings;
use semver::Version;

/// Só URLs deste prefixo são abertas no navegador.
pub const RELEASES_URL_PREFIX: &str = "https://github.com/carvalhosauro/ray-task/releases/";
/// Primeira verificação depois de abrir (não pesa na inicialização).
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(5);
/// O app pode ficar aberto por dias: reavalia a cada 6 h (a verificação em si é diária).
pub const RECHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_INTERVAL_HOURS: i64 = 24;
const EXCERPT_LINES: usize = 5;
const EXCERPT_WIDTH: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: Version,
    pub url: String,
    /// Resumo do changelog (pode ser vazio).
    pub notes: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("rede: {0}")]
    Network(String),
    #[error("resposta inválida: {0}")]
    Invalid(String),
    #[error("URL fora do repositório: {0}")]
    ForeignUrl(String),
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("versão do Cargo.toml é semver")
}

pub fn parse_release(json: &str) -> Result<Release, UpdateError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| UpdateError::Invalid(e.to_string()))?;
    let tag = value["tag_name"].as_str().ok_or_else(|| UpdateError::Invalid("sem tag_name".into()))?;
    let version = Version::parse(tag.trim_start_matches('v')).map_err(|e| UpdateError::Invalid(format!("tag '{tag}': {e}")))?;
    let url = value["html_url"].as_str().ok_or_else(|| UpdateError::Invalid("sem html_url".into()))?;
    if !url.starts_with(RELEASES_URL_PREFIX) {
        return Err(UpdateError::ForeignUrl(url.to_string()));
    }
    let notes = notes_excerpt(value["body"].as_str().unwrap_or_default());
    Ok(Release { version, url: url.to_string(), notes })
}

/// `candidate` é maior que a versão atual e que a versão dispensada (se legível).
pub fn is_newer(current: &Version, candidate: &Version, dismissed: Option<&str>) -> bool {
    candidate > current && dismissed.and_then(|d| Version::parse(d).ok()).is_none_or(|d| candidate > &d)
}

/// Verificação automática: ligada e (nunca verificou, passou um dia, ou o relógio voltou).
pub fn should_check(settings: &Settings, now: DateTime<Utc>) -> bool {
    settings.update_check
        && settings.update_last_check.is_none_or(|last| last > now || now - last >= TimeDelta::hours(CHECK_INTERVAL_HOURS))
}

/// O corpo do release publicado pelo `dist` é o changelog seguido de `## Install …`: lê só o que
/// vem antes, sem títulos nem blocos de código; marcadores viram `• `.
pub fn notes_excerpt(body: &str) -> String {
    let mut in_code = false;
    let mut lines = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with("## Install") {
            break;
        }
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let text = match line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            Some(item) => format!("• {}", item.trim()),
            None => line.to_string(),
        };
        lines.push(cut(&text));
        if lines.len() == EXCERPT_LINES {
            break;
        }
    }
    lines.join("\n")
}

fn cut(text: &str) -> String {
    if text.chars().count() <= EXCERPT_WIDTH {
        return text.to_string();
    }
    let mut out: String = text.chars().take(EXCERPT_WIDTH - 1).collect();
    out.push('…');
    out
}

pub fn install_command_for(windows: bool) -> &'static str {
    if windows {
        "powershell -ExecutionPolicy Bypass -c \"irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex\""
    } else {
        "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh"
    }
}

pub fn install_command() -> &'static str {
    install_command_for(cfg!(windows))
}

const API_URL: &str = "https://api.github.com/repos/carvalhosauro/ray-task/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);

/// GET do último release (bloqueia: chame fora do event loop). Devolve o corpo cru.
pub fn fetch() -> Result<String, UpdateError> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).build().into();
    let mut response = agent
        .get(API_URL)
        .header("User-Agent", concat!("ray-task/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    response.body_mut().read_to_string().map_err(|e| UpdateError::Network(e.to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, TimeDelta};

    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/github-release.json");

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn at(s: &str) -> DateTime<Utc> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc()
    }

    #[test]
    fn parses_the_github_release() {
        let r = parse_release(FIXTURE).unwrap();
        assert_eq!(r.version, v("0.2.0"));
        assert_eq!(r.url, "https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0");
        assert_eq!(r.notes, "• add a trash button to task rows\n• settings page with light and dark theme");
    }

    #[test]
    fn rejects_urls_outside_the_repository() {
        let json = FIXTURE.replace("https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0", "https://evil.example/x");
        assert!(matches!(parse_release(&json), Err(UpdateError::ForeignUrl(_))));
    }

    #[test]
    fn rejects_bad_json_and_bad_tags() {
        assert!(matches!(parse_release("not json"), Err(UpdateError::Invalid(_))));
        assert!(matches!(parse_release("{}"), Err(UpdateError::Invalid(_))));
        let json = FIXTURE.replace("\"v0.2.0\"", "\"nightly\"");
        assert!(matches!(parse_release(&json), Err(UpdateError::Invalid(_))));
    }

    #[test]
    fn is_newer_compares_semver() {
        assert!(is_newer(&v("0.1.0"), &v("0.2.0"), None));
        assert!(is_newer(&v("0.9.0"), &v("0.10.0"), None), "semver, não texto");
        assert!(!is_newer(&v("0.2.0"), &v("0.2.0"), None));
        assert!(!is_newer(&v("0.2.0"), &v("0.1.9"), None));
    }

    #[test]
    fn is_newer_respects_the_dismissed_version() {
        assert!(!is_newer(&v("0.1.0"), &v("0.2.0"), Some("0.2.0")));
        assert!(is_newer(&v("0.1.0"), &v("0.3.0"), Some("0.2.0")), "versão maior volta a avisar");
        assert!(!is_newer(&v("0.1.0"), &v("0.2.0"), Some("0.3.0")));
        assert!(is_newer(&v("0.1.0"), &v("0.2.0"), Some("lixo")), "dispensada ilegível não esconde");
    }

    #[test]
    fn should_check_once_a_day_when_enabled() {
        let mut s = Settings::default();
        assert!(should_check(&s, at("2026-10-06 10:00")), "nunca verificou");
        s.update_last_check = Some(at("2026-10-06 10:00"));
        assert!(!should_check(&s, at("2026-10-07 09:59")));
        assert!(should_check(&s, at("2026-10-07 10:00")));
        s.update_check = false;
        assert!(!should_check(&s, at("2026-10-09 10:00")));
    }

    #[test]
    fn should_check_when_last_check_is_in_the_future() {
        let s = Settings { update_last_check: Some(at("2026-10-06 10:00") + TimeDelta::days(30)), ..Settings::default() };
        assert!(should_check(&s, at("2026-10-06 10:00")), "relógio voltou: não trava para sempre");
    }

    #[test]
    fn excerpt_keeps_only_the_changelog() {
        let dist_only = "## Install ray-task 0.1.0\n\n### Install prebuilt binaries via shell script\n\n```sh\ncurl x | sh\n```\n";
        assert_eq!(notes_excerpt(dist_only), "");
        let long = format!("- {}\n", "a".repeat(100));
        assert_eq!(notes_excerpt(&long), format!("• {}…", "a".repeat(77)));
        let many = (1..=8).map(|i| format!("* item {i}")).collect::<Vec<_>>().join("\n");
        assert_eq!(notes_excerpt(&many).lines().count(), 5);
        assert_eq!(notes_excerpt("Plain line\n\n# Title\n"), "Plain line");
    }

    #[test]
    fn install_commands_match_the_readme() {
        assert_eq!(
            install_command_for(false),
            "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.sh | sh"
        );
        assert_eq!(
            install_command_for(true),
            "powershell -ExecutionPolicy Bypass -c \"irm https://github.com/carvalhosauro/ray-task/releases/latest/download/ray-task-installer.ps1 | iex\""
        );
        let readme = include_str!("../../../README.md");
        assert!(readme.contains(install_command_for(false)));
        assert!(readme.contains(install_command_for(true)));
    }
}
