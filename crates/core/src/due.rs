use chrono::NaiveDateTime;

use crate::model::Due;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueStatus {
    NoDate,
    Overdue { days: i64 },
    Today,
    /// Negativo = a hora já passou.
    TodayAt { minutes_until: i64 },
    Future,
}

pub fn due_status(due: Option<Due>, now: NaiveDateTime) -> DueStatus {
    let Some(due) = due else { return DueStatus::NoDate };
    let today = now.date();
    if due.date < today {
        return DueStatus::Overdue { days: (today - due.date).num_days() };
    }
    if due.date > today {
        return DueStatus::Future;
    }
    match due.time {
        Some(time) => DueStatus::TodayAt { minutes_until: (time - now.time()).num_minutes() },
        None => DueStatus::Today,
    }
}
