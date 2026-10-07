use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

pub type ProjectId = i64;
pub type TaskId = i64;
pub type TagId = i64;

/// Paleta de projetos (mesma ordem do menu "Cor").
pub const PROJECT_COLORS: [&str; 7] = ["#0A84FF", "#FF9F0A", "#BF5AF2", "#30D158", "#FF453A", "#64D2FF", "#FFD60A"];
pub const TAG_COLOR: &str = "#8E8E93";

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub id: TagId,
    pub project_id: ProjectId,
    pub name: String,
    pub color: String,
}

/// Prazo: dia obrigatório, hora opcional (a hora nunca existe sem o dia).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Due {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: TaskId,
    /// `None` = Entrada.
    pub project_id: Option<ProjectId>,
    pub title: String,
    pub notes: String,
    pub due: Option<Due>,
    pub completed_at: Option<DateTime<Utc>>,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<TagId>,
}

impl Task {
    pub fn is_done(&self) -> bool {
        self.completed_at.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub projects: Vec<Project>,
    pub tags: Vec<Tag>,
    pub tasks: Vec<Task>,
    pub settings: Settings,
}

pub const SETTING_THEME: &str = "theme";
pub const SETTING_UPDATE_CHECK: &str = "update_check";
pub const SETTING_UPDATE_LAST_CHECK: &str = "update_last_check";
pub const SETTING_UPDATE_DISMISSED: &str = "update_dismissed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark].into_iter().find(|m| m.as_str() == s)
    }
}

/// Preferências do app. Não entram no desfazer.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub theme: ThemeMode,
    pub update_check: bool,
    /// Última verificação de atualização que deu certo.
    pub update_last_check: Option<DateTime<Utc>>,
    /// Versão que o usuário pediu para não ser avisado.
    pub update_dismissed: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: ThemeMode::System, update_check: true, update_last_check: None, update_dismissed: None }
    }
}

impl Settings {
    /// Aplica uma linha da tabela `settings`. Chave desconhecida é ignorada (banco de uma versão
    /// mais nova); valor inválido é erro e não muda nada.
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            SETTING_THEME => self.theme = ThemeMode::parse(value).ok_or_else(|| format!("tema '{value}'"))?,
            SETTING_UPDATE_CHECK => {
                self.update_check = match value {
                    "1" => true,
                    "0" => false,
                    _ => return Err(format!("booleano '{value}'")),
                }
            }
            SETTING_UPDATE_LAST_CHECK => {
                let when = DateTime::parse_from_rfc3339(value).map_err(|e| format!("data/hora '{value}': {e}"))?;
                self.update_last_check = Some(when.with_timezone(&Utc));
            }
            SETTING_UPDATE_DISMISSED => self.update_dismissed = Some(value.to_string()),
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Today,
    Upcoming,
    Inbox,
    Project(ProjectId),
}

/// Junta qualquer sequência de espaços/quebras de linha num espaço só e remove as pontas.
pub fn normalize_name(raw: &str) -> Option<String> {
    let joined = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (!joined.is_empty()).then_some(joined)
}

/// Como `normalize_name`, mas aceita o `#` que o usuário costuma digitar.
pub fn normalize_tag_name(raw: &str) -> Option<String> {
    normalize_name(raw.trim().trim_start_matches('#'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_trims_and_joins_lines() {
        assert_eq!(normalize_name("  Pagar\n boleto  "), Some("Pagar boleto".to_string()));
        assert_eq!(normalize_name("a\t\tb"), Some("a b".to_string()));
    }

    #[test]
    fn normalize_name_rejects_blank() {
        assert_eq!(normalize_name(" \n\t "), None);
        assert_eq!(normalize_name(""), None);
    }

    #[test]
    fn tag_name_drops_leading_hash() {
        assert_eq!(normalize_tag_name(" #dev "), Some("dev".to_string()));
        assert_eq!(normalize_tag_name("#"), None);
    }

    #[test]
    fn settings_apply_known_keys() {
        let mut s = Settings::default();
        s.apply("theme", "dark").unwrap();
        s.apply("update_check", "0").unwrap();
        s.apply("update_last_check", "2026-10-06T10:00:00Z").unwrap();
        s.apply("update_dismissed", "0.2.0").unwrap();
        assert_eq!(s.theme, ThemeMode::Dark);
        assert!(!s.update_check);
        assert_eq!(s.update_last_check.unwrap().to_rfc3339(), "2026-10-06T10:00:00+00:00");
        assert_eq!(s.update_dismissed.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn settings_reject_invalid_values_and_ignore_unknown_keys() {
        let mut s = Settings::default();
        assert!(s.apply("theme", "purple").is_err());
        assert!(s.apply("update_check", "maybe").is_err());
        assert!(s.apply("update_last_check", "ontem").is_err());
        assert!(s.apply("some_future_key", "x").is_ok());
        assert_eq!(s, Settings::default());
    }
}
