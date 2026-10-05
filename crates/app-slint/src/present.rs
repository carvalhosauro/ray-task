use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, NaiveTime};
use ray_core::{due_status, Due, DueStatus, View};

const WEEKDAY_SHORT: [&str; 7] = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
const WEEKDAY_NAME: [&str; 7] = ["Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado", "Domingo"];
const WEEKDAY_LONG: [&str; 7] = ["Segunda-feira", "Terça-feira", "Quarta-feira", "Quinta-feira", "Sexta-feira", "Sábado", "Domingo"];
const MONTH_SHORT: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
const MONTH_NAME: [&str; 12] =
    ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Normal,
    Soon,
    Late,
}

impl Tone {
    pub fn as_int(self) -> i32 {
        match self {
            Tone::Normal => 0,
            Tone::Soon => 1,
            Tone::Late => 2,
        }
    }
}

fn weekday(d: NaiveDate) -> usize {
    d.weekday().num_days_from_monday() as usize
}

fn month(d: NaiveDate) -> usize {
    d.month0() as usize
}

/// "Qui, 8 out" (com o ano se não for o ano corrente).
pub fn short_date(d: NaiveDate, today: NaiveDate) -> String {
    let base = format!("{}, {} {}", WEEKDAY_SHORT[weekday(d)], d.day(), MONTH_SHORT[month(d)]);
    if d.year() == today.year() {
        base
    } else {
        format!("{base} {}", d.year())
    }
}

pub fn date_label(d: NaiveDate, today: NaiveDate) -> String {
    match (d - today).num_days() {
        -1 => "Ontem".into(),
        0 => "Hoje".into(),
        1 => "Amanhã".into(),
        _ => short_date(d, today),
    }
}

/// Subtítulo da visão Hoje: "Segunda-feira, 5 de outubro".
pub fn header_date(today: NaiveDate) -> String {
    format!("{}, {} de {}", WEEKDAY_LONG[weekday(today)], today.day(), MONTH_NAME[month(today)])
}

/// Cabeçalho de grupo em Próximos: (título, complemento).
pub fn group_header(d: NaiveDate, today: NaiveDate) -> (String, String) {
    let small = format!("{} {}", d.day(), MONTH_SHORT[month(d)]);
    match (d - today).num_days() {
        1 => ("Amanhã".into(), format!("{}, {small}", WEEKDAY_SHORT[weekday(d)].to_lowercase())),
        2..=6 => (WEEKDAY_NAME[weekday(d)].into(), small),
        _ => (short_date(d, today), String::new()),
    }
}

fn hhmm(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

fn relative(minutes: i64) -> String {
    if minutes < 60 {
        format!("{minutes} min")
    } else {
        format!("{} h", minutes / 60)
    }
}

/// Hora da tarefa com o indicador relativo quando for hoje (vazio se não houver hora).
pub fn time_label(due: Option<Due>, now: NaiveDateTime) -> (String, Tone) {
    let Some(time) = due.and_then(|d| d.time) else { return (String::new(), Tone::Normal) };
    match due_status(due, now) {
        DueStatus::TodayAt { minutes_until: 0 } => (format!("{} · agora", hhmm(time)), Tone::Late),
        DueStatus::TodayAt { minutes_until } if minutes_until < 0 => {
            (format!("{} · há {}", hhmm(time), relative(-minutes_until)), Tone::Late)
        }
        DueStatus::TodayAt { minutes_until } if minutes_until <= 60 => {
            (format!("{} · em {minutes_until} min", hhmm(time)), Tone::Soon)
        }
        DueStatus::Overdue { .. } => (hhmm(time), Tone::Late),
        _ => (hhmm(time), Tone::Normal),
    }
}

/// Texto à direita de uma linha recolhida.
pub fn meta(due: Option<Due>, now: NaiveDateTime, view: View) -> (String, Tone) {
    let today = now.date();
    match (due, due_status(due, now)) {
        (_, DueStatus::NoDate) | (None, _) => (String::new(), Tone::Normal),
        (Some(d), DueStatus::Overdue { .. }) => (date_label(d.date, today), Tone::Late),
        (_, DueStatus::TodayAt { .. }) => time_label(due, now),
        (_, DueStatus::Today) if view == View::Today => (String::new(), Tone::Normal),
        (_, DueStatus::Today) => ("Hoje".into(), Tone::Normal),
        (Some(d), DueStatus::Future) => {
            let time = d.time.map(hhmm);
            if view == View::Upcoming {
                (time.unwrap_or_default(), Tone::Normal)
            } else {
                match time {
                    Some(t) => (format!("{} · {t}", date_label(d.date, today)), Tone::Normal),
                    None => (date_label(d.date, today), Tone::Normal),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalDay {
    /// 0 = célula vazia antes do dia 1.
    pub day: u32,
    pub date: Option<NaiveDate>,
    pub today: bool,
    pub selected: bool,
    pub past: bool,
}

pub fn first_of_month(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("dia 1 existe")
}

pub fn shift_month(first: NaiveDate, delta: i32) -> NaiveDate {
    let shifted = if delta >= 0 {
        first.checked_add_months(Months::new(delta as u32))
    } else {
        first.checked_sub_months(Months::new(delta.unsigned_abs()))
    };
    shifted.unwrap_or(first)
}

pub fn month_title(first: NaiveDate) -> String {
    let name = MONTH_NAME[month(first)];
    let mut chars = name.chars();
    let capitalized: String = chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default();
    format!("{capitalized} {}", first.year())
}

/// Grade do mês começando no domingo (como no mockup: D S T Q Q S S).
pub fn calendar(first: NaiveDate, today: NaiveDate, selected: Option<NaiveDate>) -> Vec<CalDay> {
    let blanks = first.weekday().num_days_from_sunday();
    let mut cells: Vec<CalDay> =
        (0..blanks).map(|_| CalDay { day: 0, date: None, today: false, selected: false, past: false }).collect();
    let mut d = first;
    while d.month() == first.month() {
        cells.push(CalDay { day: d.day(), date: Some(d), today: d == today, selected: Some(d) == selected, past: d < today });
        d = d.succ_opt().expect("data válida");
    }
    cells
}

/// Aceita "9", "9:5", "09:05". Hora 0–23, minuto 0–59, até 2 dígitos cada.
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim();
    let (h, m) = text.split_once(':').unwrap_or((text, "0"));
    if h.is_empty() || h.len() > 2 || m.is_empty() || m.len() > 2 {
        return None;
    }
    NaiveTime::from_hms_opt(h.parse().ok()?, m.parse().ok()?, 0)
}

/// Próxima segunda-feira estritamente depois de hoje.
pub fn next_monday(today: NaiveDate) -> NaiveDate {
    today + Days::new(7 - weekday(today) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use ray_core::{Due, View};

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn now(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }
    fn due(date: &str, time: Option<&str>) -> Option<Due> {
        Some(Due { date: d(date), time: time.map(|t| NaiveTime::parse_from_str(t, "%H:%M").unwrap()) })
    }
    const TODAY: &str = "2026-10-05"; // segunda-feira

    #[test]
    fn relative_date_labels() {
        let today = d(TODAY);
        assert_eq!(date_label(d("2026-10-04"), today), "Ontem");
        assert_eq!(date_label(d("2026-10-05"), today), "Hoje");
        assert_eq!(date_label(d("2026-10-06"), today), "Amanhã");
        assert_eq!(date_label(d("2026-10-08"), today), "Qui, 8 out");
        assert_eq!(date_label(d("2026-09-20"), today), "Dom, 20 set");
        assert_eq!(date_label(d("2027-01-01"), today), "Sex, 1 jan 2027");
    }

    #[test]
    fn header_and_group_labels() {
        let today = d(TODAY);
        assert_eq!(header_date(today), "Segunda-feira, 5 de outubro");
        assert_eq!(header_date(d("2026-10-10")), "Sábado, 10 de outubro");
        assert_eq!(group_header(d("2026-10-06"), today), ("Amanhã".to_string(), "ter, 6 out".to_string()));
        assert_eq!(group_header(d("2026-10-08"), today), ("Quinta".to_string(), "8 out".to_string()));
        assert_eq!(group_header(d("2026-10-20"), today), ("Ter, 20 out".to_string(), String::new()));
    }

    #[test]
    fn time_labels_by_distance() {
        let n = now("2026-10-05 13:35");
        assert_eq!(time_label(due(TODAY, Some("14:00")), n), ("14:00 · em 25 min".to_string(), Tone::Soon));
        assert_eq!(time_label(due(TODAY, Some("13:00")), n), ("13:00 · há 35 min".to_string(), Tone::Late));
        assert_eq!(time_label(due(TODAY, Some("11:00")), n), ("11:00 · há 2 h".to_string(), Tone::Late));
        assert_eq!(time_label(due(TODAY, Some("13:35")), n), ("13:35 · agora".to_string(), Tone::Late));
        assert_eq!(time_label(due(TODAY, Some("18:00")), n), ("18:00".to_string(), Tone::Normal));
        assert_eq!(time_label(due("2026-10-08", Some("10:00")), n), ("10:00".to_string(), Tone::Normal));
        assert_eq!(time_label(due(TODAY, None), n), (String::new(), Tone::Normal));
    }

    #[test]
    fn meta_depends_on_view() {
        let n = now("2026-10-05 13:35");
        assert_eq!(meta(due("2026-10-04", None), n, View::Today), ("Ontem".to_string(), Tone::Late));
        assert_eq!(meta(due(TODAY, None), n, View::Today), (String::new(), Tone::Normal));
        assert_eq!(meta(due(TODAY, None), n, View::Inbox), ("Hoje".to_string(), Tone::Normal));
        assert_eq!(meta(due("2026-10-08", Some("10:00")), n, View::Upcoming), ("10:00".to_string(), Tone::Normal));
        assert_eq!(meta(due("2026-10-08", Some("10:00")), n, View::Project(1)), ("Qui, 8 out · 10:00".to_string(), Tone::Normal));
        assert_eq!(meta(None, n, View::Inbox), (String::new(), Tone::Normal));
    }

    #[test]
    fn october_2026_calendar_starts_on_thursday() {
        let cells = calendar(d("2026-10-01"), d(TODAY), Some(d("2026-10-08")));
        assert_eq!(cells.len(), 4 + 31);
        assert!(cells[..4].iter().all(|c| c.day == 0 && c.date.is_none()));
        assert_eq!(cells[4].day, 1);
        assert!(cells[4].past);
        assert!(cells[8].today && cells[8].day == 5);
        assert!(cells[11].selected && cells[11].day == 8);
        assert_eq!(month_title(d("2026-10-01")), "Outubro 2026");
        assert_eq!(month_title(shift_month(d("2026-10-01"), 3)), "Janeiro 2027");
        assert_eq!(shift_month(d("2026-10-01"), -10), d("2025-12-01"));
        assert_eq!(first_of_month(d("2026-10-17")), d("2026-10-01"));
    }

    #[test]
    fn parse_time_accepts_loose_formats() {
        let t = |h, m| Some(NaiveTime::from_hms_opt(h, m, 0).unwrap());
        assert_eq!(parse_time("9"), t(9, 0));
        assert_eq!(parse_time(" 9:5 "), t(9, 5));
        assert_eq!(parse_time("09:05"), t(9, 5));
        assert_eq!(parse_time("23:59"), t(23, 59));
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time("12:60"), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time("1230"), None);
        assert_eq!(parse_time(""), None);
    }

    #[test]
    fn next_monday_is_always_in_the_future() {
        assert_eq!(next_monday(d("2026-10-05")), d("2026-10-12")); // segunda
        assert_eq!(next_monday(d("2026-10-11")), d("2026-10-12")); // domingo
        assert_eq!(next_monday(d("2026-10-07")), d("2026-10-12")); // quarta
    }
}
