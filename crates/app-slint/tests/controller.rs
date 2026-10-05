use std::cell::RefCell;
use std::rc::Rc;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use ray_core::{FixedClock, Snapshot, Store, View, WriteOp};
use ray_task::controller::{Controller, QuickDate};
use ray_task::present::Tone;

struct Fixture {
    c: Controller,
    clock: FixedClock,
    sent: Rc<RefCell<Vec<WriteOp>>>,
}

fn at(now: &str) -> Fixture {
    let clock = FixedClock::at(now);
    let sent = Rc::new(RefCell::new(Vec::new()));
    let sink = sent.clone();
    let store = Store::new(Snapshot::default(), Box::new(clock.clone()));
    let c = Controller::new(store, Box::new(move |ops| sink.borrow_mut().extend(ops)));
    Fixture { c, clock, sent }
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn new_task_in_today_is_due_today_and_persisted() {
    let mut f = at("2026-10-05 13:35");
    f.c.start_adding();
    let id = f.c.commit_new("Comprar pão").unwrap();
    assert!(f.c.adding, "continua no modo de captura para a próxima");
    let rows = f.c.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert!(rows[0].fresh);
    assert_eq!(f.c.store.task(id).unwrap().due.unwrap().date, d("2026-10-05"));
    assert_eq!(f.sent.borrow().len(), 1);
    assert_eq!(f.c.nav_rows()[0].count, 1);
}

#[test]
fn completed_task_lingers_then_leaves() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    assert_eq!(f.c.toggle(id), Ok(true));
    let row = &f.c.rows()[0];
    assert!(row.done && !row.leaving);
    assert_eq!(f.c.nav_rows()[0].count, 0);
    assert!(f.c.start_leaving(id));
    assert!(f.c.rows()[0].leaving);
    f.c.finish_leaving(id);
    assert!(f.c.rows().is_empty());
}

#[test]
fn undo_during_linger_keeps_the_task() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.toggle(id).unwrap();
    assert!(f.c.undo());
    assert!(!f.c.start_leaving(id));
    let rows = f.c.rows();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].done && !rows[0].leaving);
}

#[test]
fn delete_happens_only_after_the_animation() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    assert!(f.c.begin_delete(id));
    assert!(f.c.rows()[0].leaving);
    assert!(f.c.store.task(id).is_some());
    f.c.finish_delete(id).unwrap();
    assert!(f.c.rows().is_empty());
    assert!(f.c.undo());
    assert_eq!(f.c.rows().len(), 1);
}

#[test]
fn upcoming_rows_have_one_header_per_day() {
    let mut f = at("2026-10-05 13:35");
    f.c.select_view(View::Upcoming);
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    let c = f.c.commit_new("c").unwrap();
    f.c.set_date(c, Some(d("2026-10-08"))).unwrap();
    let rows = f.c.rows();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![a, b, c]);
    assert_eq!((rows[0].group_header.as_str(), rows[0].group_sub.as_str()), ("Amanhã", "ter, 6 out"));
    assert_eq!(rows[1].group_header, "");
    assert_eq!((rows[2].group_header.as_str(), rows[2].group_sub.as_str()), ("Quinta", "8 out"));
    assert!(rows.iter().all(|r| r.show_project && r.project_name == "Entrada"));
}

#[test]
fn today_rows_show_relative_time() {
    let mut f = at("2026-10-05 13:35");
    let past = f.c.commit_new("daily").unwrap();
    let soon = f.c.commit_new("boleto").unwrap();
    assert_eq!(f.c.set_time(past, "13:00"), Ok(true));
    assert_eq!(f.c.set_time(soon, "14:00"), Ok(true));
    let rows = f.c.rows();
    assert_eq!((rows[0].meta.as_str(), rows[0].tone), ("13:00 · há 35 min", Tone::Late));
    assert_eq!((rows[1].meta.as_str(), rows[1].tone), ("14:00 · em 25 min", Tone::Soon));
    assert!(f.c.nav_rows()[0].alert);
}

#[test]
fn set_time_accepts_loose_formats_and_keeps_previous_on_garbage() {
    let mut f = at("2026-10-05 13:35");
    f.c.select_view(View::Inbox);
    let id = f.c.commit_new("x").unwrap();
    assert_eq!(f.c.set_time(id, "9"), Ok(true));
    let due = f.c.store.task(id).unwrap().due.unwrap();
    assert_eq!(due.date, d("2026-10-05"), "sem data: a hora usa hoje");
    assert_eq!(due.time, NaiveTime::from_hms_opt(9, 0, 0));
    assert_eq!(f.c.set_time(id, "abc"), Ok(false));
    assert_eq!(f.c.store.task(id).unwrap().due.unwrap().time, NaiveTime::from_hms_opt(9, 0, 0));
    assert_eq!(f.c.set_time(id, ""), Ok(true));
    assert_eq!(f.c.store.task(id).unwrap().due.unwrap().time, None);
    f.c.set_date(id, None).unwrap();
    assert_eq!(f.c.store.task(id).unwrap().due, None);
}

#[test]
fn quick_dates() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.set_time(id, "10:00").unwrap();
    f.c.quick_date(id, QuickDate::NextWeek).unwrap();
    let due = f.c.store.task(id).unwrap().due.unwrap();
    assert_eq!(due.date, d("2026-10-12"));
    assert_eq!(due.time, NaiveTime::from_hms_opt(10, 0, 0), "trocar o dia mantém a hora");
    f.c.quick_date(id, QuickDate::Tomorrow).unwrap();
    assert_eq!(f.c.store.task(id).unwrap().due.unwrap().date, d("2026-10-06"));
    f.c.quick_date(id, QuickDate::Clear).unwrap();
    assert_eq!(f.c.store.task(id).unwrap().due, None);
    assert_eq!(f.c.next_week_hint(), "Seg, 12 out");
}

#[test]
fn tags_and_moves() {
    let mut f = at("2026-10-05 13:35");
    let p = f.c.create_project("ray-task").unwrap();
    assert_eq!(f.c.view, View::Project(p), "criar projeto já abre o projeto");
    let id = f.c.commit_new("Revisar PR").unwrap();
    f.c.add_tag_by_name(id, "#Dev").unwrap();
    f.c.add_tag_by_name(id, "docs").unwrap();
    assert_eq!(f.c.rows()[0].tags.iter().map(|t| t.1.as_str()).collect::<Vec<_>>(), vec!["Dev", "docs"]);
    let other = f.c.commit_new("Outra").unwrap();
    assert_eq!(f.c.tag_suggestion(other, "d"), "Dev");
    assert_eq!(f.c.tag_suggestion(other, "do"), "docs");
    assert_eq!(f.c.tag_suggestion(id, "d"), "", "não sugere tag que a tarefa já tem");
    f.c.remove_last_tag(id).unwrap();
    assert_eq!(f.c.rows()[0].tags.len(), 1);
    assert!(f.c.move_needs_confirm(id, None));
    assert!(!f.c.move_needs_confirm(other, None));
    f.c.move_task(id, None).unwrap();
    assert!(f.c.store.task(id).unwrap().tags.is_empty());
    let targets = f.c.move_targets();
    assert_eq!(targets[0], (None, "Entrada".to_string(), "#8E8E93".to_string()));
    assert_eq!(targets[1].0, Some(p));
}

#[test]
fn escape_unwinds_one_layer_at_a_time() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.toggle_filter();
    f.c.set_filter("x");
    f.c.toggle_expand(id);
    f.c.open_popover(id);
    f.c.adding = true; // start_adding() fecharia a tarefa; aqui queremos as quatro camadas abertas
    f.c.escape();
    assert!(!f.c.adding);
    f.c.escape();
    assert!(f.c.popover_task.is_none() && f.c.expanded.is_some());
    f.c.escape();
    assert!(f.c.expanded.is_none() && f.c.filter_open);
    f.c.escape();
    assert!(!f.c.filter_open && f.c.filter.is_empty());
    assert_eq!(f.c.selected, Some(id));
    f.c.escape();
    assert_eq!(f.c.selected, None);
}

#[test]
fn filter_limits_rows_and_explains_empty_result() {
    let mut f = at("2026-10-05 13:35");
    f.c.commit_new("Pagar boleto").unwrap();
    f.c.commit_new("Ligar pro João").unwrap();
    f.c.set_filter("joão");
    assert_eq!(f.c.rows().len(), 1);
    f.c.set_filter("xyz");
    assert!(f.c.rows().is_empty());
    assert_eq!(f.c.empty_text(), "Nada encontrado para “xyz”.");
    f.c.set_filter("");
    f.c.select_view(View::Upcoming);
    assert_eq!(f.c.empty_text(), "Nada por aqui. Ctrl+N para capturar uma tarefa.");
}

#[test]
fn tick_pulses_only_tasks_that_just_became_due() {
    let mut f = at("2026-10-05 13:59");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.set_time(a, "14:00").unwrap();
    f.c.set_time(b, "15:00").unwrap();
    let previous = f.c.store.now();
    f.clock.set("2026-10-05 14:00");
    assert_eq!(f.c.tick(previous), vec![a]);
    assert!(f.c.rows().iter().find(|r| r.id == a).unwrap().pulse);
    f.c.clear_pulse();
    assert!(f.c.rows().iter().all(|r| !r.pulse));

    let previous = NaiveDateTime::parse_from_str("2026-10-05 23:59", "%Y-%m-%d %H:%M").unwrap();
    f.clock.set("2026-10-06 00:00");
    assert!(f.c.tick(previous).is_empty(), "virada do dia não pulsa nada");
}

#[test]
fn deleting_the_open_project_returns_to_today() {
    let mut f = at("2026-10-05 13:35");
    let p = f.c.create_project("Casa").unwrap();
    f.c.commit_new("x").unwrap();
    f.c.commit_new("y").unwrap();
    assert_eq!(f.c.project_task_count(p), 2);
    assert_eq!(f.c.subtitle(), "2 pendentes");
    assert_eq!(f.c.delete_project(p), Ok(2));
    assert_eq!(f.c.view, View::Today);
    assert_eq!(f.c.title(), "Hoje");
    assert_eq!(f.c.nav_rows().len(), 3);
}

#[test]
fn project_view_can_show_completed_section() {
    let mut f = at("2026-10-05 13:35");
    f.c.create_project("Casa").unwrap();
    let a = f.c.commit_new("a").unwrap();
    f.c.commit_new("b").unwrap();
    f.c.toggle(a).unwrap();
    f.c.start_leaving(a);
    f.c.finish_leaving(a);
    assert_eq!(f.c.done_toggle_label(), "Mostrar concluídas (1)");
    f.c.toggle_show_done();
    let rows = f.c.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].group_header, "Concluídas");
    assert!(rows[1].done);
    assert_eq!(f.c.done_toggle_label(), "Ocultar concluídas (1)");
}

#[test]
fn arrow_selection_moves_and_clamps() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(a));
    f.c.move_selection(1);
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(b));
    f.c.move_selection(-5);
    assert_eq!(f.c.selected, Some(a));
    assert_eq!(f.c.focused_task(), Some(a));
}

#[test]
fn nav_and_titles() {
    let mut f = at("2026-10-05 13:35");
    assert_eq!(f.c.title(), "Hoje");
    assert_eq!(f.c.subtitle(), "Segunda-feira, 5 de outubro");
    f.c.create_project("Casa").unwrap();
    assert_eq!(f.c.selected_nav(), 3);
    f.c.set_project_color(f.c.nav_rows()[3].view_project().unwrap(), 3).unwrap();
    assert_eq!(f.c.nav_rows()[3].color, "#30D158");
    f.c.select_view(View::Inbox);
    assert_eq!((f.c.title().as_str(), f.c.subtitle().as_str()), ("Entrada", "Capture agora, organize depois"));
}

#[test]
fn calendar_follows_popover_month() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.open_popover(id);
    assert_eq!(f.c.expanded, Some(id));
    let (title, cells) = f.c.calendar();
    assert_eq!(title, "Outubro 2026");
    assert!(cells.iter().any(|c| c.selected && c.day == 5));
    f.c.shift_popover_month(1);
    assert_eq!(f.c.calendar().0, "Novembro 2026");
    assert_eq!(f.c.popover_time(), "");
    f.c.close_popover();
    assert!(f.c.popover_task.is_none());
}

#[test]
fn undo_after_leaving_started_clears_animation_state() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.toggle(id).unwrap();
    assert!(f.c.start_leaving(id));
    assert!(f.c.undo());
    let rows = f.c.rows();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].done && !rows[0].leaving);
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(id));
}

// ---------- foco obsoleto (I4) e seleção após desfazer ----------

#[test]
fn moving_a_task_out_of_the_view_clears_focus() {
    let mut f = at("2026-10-05 13:35");
    let p = f.c.store.create_project("Casa").unwrap();
    f.c.select_view(View::Inbox);
    let id = f.c.commit_new("Comprar pão").unwrap();
    f.c.toggle_expand(id);
    assert_eq!((f.c.expanded, f.c.selected), (Some(id), Some(id)));
    f.c.move_task(id, Some(p)).unwrap();
    assert_eq!((f.c.expanded, f.c.selected, f.c.focused_task()), (None, None, None));
}

#[test]
fn clearing_the_date_in_today_clears_focus_and_popover() {
    let mut f = at("2026-10-05 13:35");
    let keep = f.c.commit_new("Fica").unwrap();
    let id = f.c.commit_new("Sai").unwrap();
    f.c.open_popover(id);
    assert_eq!(f.c.popover_task, Some(id));
    f.c.quick_date(id, QuickDate::Clear).unwrap();
    assert_eq!((f.c.expanded, f.c.selected, f.c.popover_task), (None, None, None));

    // continua na visão: o foco fica
    f.c.toggle_expand(keep);
    f.c.quick_date(keep, QuickDate::Today).unwrap();
    assert_eq!((f.c.expanded, f.c.selected), (Some(keep), Some(keep)));
}

#[test]
fn deleting_the_project_of_a_focused_task_clears_focus() {
    let mut f = at("2026-10-05 13:35");
    let p = f.c.store.create_project("Casa").unwrap();
    let id = f.c.store.create_task(View::Project(p), "Limpar garagem").unwrap();
    f.c.store.set_due(id, Some(ray_core::Due { date: d("2026-10-05"), time: None })).unwrap();
    assert_eq!(f.c.rows().len(), 1, "aparece em Hoje");
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(id));
    f.c.delete_project(p).unwrap();
    assert_eq!((f.c.selected, f.c.focused_task()), (None, None));
}

#[test]
fn undoing_a_delete_selects_the_restored_task() {
    let mut f = at("2026-10-05 13:35");
    f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.commit_new("c").unwrap();
    f.c.move_selection(1);
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(b));
    assert!(f.c.begin_delete(b));
    f.c.finish_delete(b).unwrap();
    assert_eq!(f.c.selected, None);
    assert!(f.c.undo());
    assert_eq!(f.c.selected, Some(b), "a tarefa restaurada volta selecionada");
}
