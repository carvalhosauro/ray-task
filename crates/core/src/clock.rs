use std::cell::Cell;
use std::rc::Rc;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};

pub trait Clock {
    fn now_local(&self) -> NaiveDateTime;
    fn now_utc(&self) -> DateTime<Utc>;
    fn today(&self) -> NaiveDate {
        self.now_local().date()
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_local(&self) -> NaiveDateTime {
        Local::now().naive_local()
    }
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Relógio controlável para testes. Clones compartilham o mesmo instante.
#[derive(Clone)]
pub struct FixedClock(Rc<Cell<NaiveDateTime>>);

impl FixedClock {
    pub fn at(local: &str) -> Self {
        Self(Rc::new(Cell::new(parse(local))))
    }
    pub fn set(&self, local: &str) {
        self.0.set(parse(local));
    }
}

fn parse(local: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(local, "%Y-%m-%d %H:%M").expect("formato esperado: AAAA-MM-DD HH:MM")
}

impl Clock for FixedClock {
    fn now_local(&self) -> NaiveDateTime {
        self.0.get()
    }
    /// Nos testes o horário local é tratado como UTC.
    fn now_utc(&self) -> DateTime<Utc> {
        self.0.get().and_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_can_be_moved_forward() {
        let clock = FixedClock::at("2026-10-05 13:35");
        let shared = clock.clone();
        assert_eq!(clock.today().to_string(), "2026-10-05");
        shared.set("2026-10-06 00:01");
        assert_eq!(clock.today().to_string(), "2026-10-06");
        assert_eq!(clock.now_utc().to_rfc3339(), "2026-10-06T00:01:00+00:00");
    }
}
