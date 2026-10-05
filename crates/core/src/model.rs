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
}
