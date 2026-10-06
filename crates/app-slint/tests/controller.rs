use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use ray_core::{FixedClock, Snapshot, Store, ThemeMode, View, WriteOp};
use ray_task::controller::{Controller, Page, QuickDate};
use ray_task::controller::{Nav, TagKey};
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

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
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
    assert_eq!(f.c.toggle(id, ms(0)), Ok(true));
    let row = &f.c.rows()[0];
    assert!(row.done && !row.leaving);
    assert_eq!(f.c.nav_rows()[0].count, 0);
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
    f.c.advance(ms(820));
    assert!(f.c.rows().is_empty());
}

#[test]
fn undo_during_linger_keeps_the_task() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    f.c.toggle(id, ms(0)).unwrap();
    assert!(f.c.undo());
    f.c.advance(ms(1_000));
    let rows = f.c.rows();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].done && !rows[0].leaving);
}

#[test]
fn delete_happens_only_after_the_animation() {
    let mut f = at("2026-10-05 13:35");
    let id = f.c.commit_new("x").unwrap();
    assert!(f.c.begin_delete(id, ms(0)));
    assert!(f.c.rows()[0].leaving);
    assert!(f.c.store.task(id).is_some());
    assert_eq!(f.c.advance(ms(220)), vec![Ok(id)]);
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
    assert_eq!(f.c.tick(previous, ms(0)), vec![a]);
    assert!(f.c.rows().iter().find(|r| r.id == a).unwrap().pulse);
    f.c.advance(ms(700));
    assert!(f.c.rows().iter().all(|r| !r.pulse));

    let previous = NaiveDateTime::parse_from_str("2026-10-05 23:59", "%Y-%m-%d %H:%M").unwrap();
    f.clock.set("2026-10-06 00:00");
    assert!(f.c.tick(previous, ms(800)).is_empty(), "virada do dia não pulsa nada");
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
    f.c.toggle(a, ms(0)).unwrap();
    f.c.advance(ms(820));
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
    f.c.toggle(id, ms(0)).unwrap();
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
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
    assert!(f.c.begin_delete(b, ms(0)));
    assert_eq!(f.c.advance(ms(220)), vec![Ok(b)]);
    assert_eq!(f.c.selected, None);
    assert!(f.c.undo());
    assert_eq!(f.c.selected, Some(b), "a tarefa restaurada volta selecionada");
}

#[test]
fn upcoming_filter_moves_header_to_first_visible_task() {
    let mut f = at("2026-10-05 13:35");
    f.c.select_view(View::Upcoming);
    f.c.commit_new("alpha").unwrap();
    let b = f.c.commit_new("beta").unwrap();
    f.c.set_filter("beta");
    let rows = f.c.rows();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![b]);
    assert_eq!((rows[0].group_header.as_str(), rows[0].group_sub.as_str()), ("Amanhã", "ter, 6 out"));
}

#[test]
fn lingering_task_stays_out_of_completed_section() {
    let mut f = at("2026-10-05 13:35");
    f.c.create_project("Casa").unwrap();
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    let c = f.c.commit_new("c").unwrap();
    f.c.toggle(c, ms(0)).unwrap();
    f.c.advance(ms(820));
    f.c.toggle_show_done();
    f.c.toggle(a, ms(1_000)).unwrap();
    let rows = f.c.rows();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![a, b, c]);
    assert!(rows[0].done, "a está concluída mas ainda na parte aberta (lingering)");
    assert_eq!(rows.iter().map(|r| r.group_header.as_str()).collect::<Vec<_>>(), vec!["", "", "Concluídas"]);
}

#[test]
fn arrow_selection_skips_leaving_rows() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    let c = f.c.commit_new("c").unwrap();
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(a));
    assert!(f.c.begin_delete(b, ms(0)));
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(c));
    assert!(f.c.begin_delete(c, ms(0)));
    f.c.move_selection(1);
    assert_eq!(f.c.selected, Some(a), "seleção numa linha saindo recomeça do topo");
}

#[test]
fn arrow_selection_clamps_extreme_deltas() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.commit_new("b").unwrap();
    let c = f.c.commit_new("c").unwrap();
    f.c.move_selection(1);
    f.c.move_selection(1);
    f.c.move_selection(i32::MAX);
    assert_eq!(f.c.selected, Some(c));
    f.c.move_selection(i32::MIN);
    assert_eq!(f.c.selected, Some(a));
}

#[test]
fn deleting_during_linger_goes_straight_to_delete() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    assert!(f.c.begin_delete(a, ms(100)));
    assert!(f.c.rows()[0].leaving, "apagar não espera o fim do linger");
    assert_eq!(f.c.advance(ms(320)), vec![Ok(a)]);
    assert!(f.c.store.task(a).is_none());
    assert!(f.c.advance(ms(5_000)).is_empty(), "o roteiro de conclusão foi substituído");
}

#[test]
fn uncompleting_during_leaving_cancels_the_exit() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.advance(ms(600));
    assert!(f.c.rows()[0].leaving);
    assert_eq!(f.c.toggle(a, ms(700)), Ok(false));
    assert!(!f.c.rows()[0].leaving && !f.c.rows()[0].done);
    f.c.advance(ms(5_000));
    assert_eq!(f.c.rows().len(), 1);
    assert_eq!(f.c.next_anim_deadline(), None);
}

#[test]
fn recompleting_restarts_the_linger() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.toggle(a, ms(300)).unwrap();
    f.c.toggle(a, ms(400)).unwrap();
    f.c.advance(ms(900));
    assert!(!f.c.rows()[0].leaving, "linger novo de 600 ms a partir de 400");
    f.c.advance(ms(1_000));
    assert!(f.c.rows()[0].leaving);
}

#[test]
fn late_advance_finishes_every_step() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.begin_delete(b, ms(0));
    assert_eq!(f.c.advance(ms(60_000)), vec![Ok(b)]);
    assert!(f.c.rows().is_empty());
    assert_eq!(f.c.next_anim_deadline(), None);
}

#[test]
fn undo_does_not_cancel_a_pending_delete() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.begin_delete(b, ms(0));
    assert!(f.c.undo(), "desfaz a conclusão de a");
    assert_eq!(f.c.advance(ms(220)), vec![Ok(b)], "b continua sendo apagada");
    assert_eq!(f.c.rows().iter().map(|r| r.id).collect::<Vec<_>>(), vec![a]);
}

#[test]
fn next_anim_deadline_is_the_earliest() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    let b = f.c.commit_new("b").unwrap();
    assert_eq!(f.c.next_anim_deadline(), None);
    f.c.toggle(a, ms(0)).unwrap();
    assert_eq!(f.c.next_anim_deadline(), Some(ms(600)));
    f.c.begin_delete(b, ms(100));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(320)));
    f.c.advance(ms(320));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(600)));
    f.c.advance(ms(600));
    assert_eq!(f.c.next_anim_deadline(), Some(ms(820)));
}

#[test]
fn failed_delete_is_reported() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    f.c.begin_delete(a, ms(0));
    f.c.store.delete_task(a).unwrap();
    let results = f.c.advance(ms(220));
    assert!(matches!(results.as_slice(), [Err(_)]), "{results:?}");
    assert!(f.c.undo(), "desfaz a remoção feita direto no store");
    assert_eq!(f.c.selected, None, "não foi o app que apagou: o desfazer não seleciona a tarefa");
}

#[test]
fn toggle_during_delete_still_deletes() {
    let mut f = at("2026-10-05 13:35");
    let a = f.c.commit_new("a").unwrap();
    assert!(f.c.begin_delete(a, ms(0)));
    assert_eq!(f.c.toggle(a, ms(100)), Ok(true));
    assert!(f.c.rows()[0].leaving, "a linha continua saindo");
    assert_eq!(f.c.advance(ms(220)), vec![Ok(a)], "apagar vence");
    assert!(f.c.store.task(a).is_none());
}

#[test]
fn uncompleting_during_delete_still_deletes() {
    let mut f = at("2026-10-05 13:35");
    f.c.create_project("Casa").unwrap();
    let a = f.c.commit_new("a").unwrap();
    f.c.toggle(a, ms(0)).unwrap();
    f.c.advance(ms(820));
    f.c.toggle_show_done();
    assert!(f.c.begin_delete(a, ms(1_000)));
    assert_eq!(f.c.toggle(a, ms(1_100)), Ok(false));
    assert_eq!(f.c.advance(ms(1_220)), vec![Ok(a)], "apagar vence");
}

// ---------- lista de sugestões de tag (gus-combobox) ----------

/// Projeto com `names` como tags (numa tarefa à parte) e uma tarefa alvo sem tags.
fn project_with_tags(names: &[&str]) -> (Fixture, ray_core::TaskId) {
    let mut f = at("2026-10-05 13:35");
    f.c.create_project("Casa").unwrap();
    let holder = f.c.commit_new("guarda as tags").unwrap();
    for name in names {
        f.c.add_tag_by_name(holder, name).unwrap();
    }
    let id = f.c.commit_new("alvo").unwrap();
    (f, id)
}

fn tag_names(f: &Fixture, id: ray_core::TaskId) -> Vec<String> {
    f.c.rows().into_iter().find(|r| r.id == id).unwrap().tags.into_iter().map(|t| t.1).collect()
}

fn key(handled: bool, text: &str) -> TagKey {
    TagKey { handled, text: text.to_string(), added: false }
}

fn added() -> TagKey {
    TagKey { handled: true, text: String::new(), added: true }
}

#[test]
fn tag_options_exclude_the_tasks_tags_and_rank_starts_with_first() {
    let (mut f, id) = project_with_tags(&["casa", "carro", "ui-kit", "kit"]);
    f.c.add_tag_by_name(id, "kit").unwrap();
    assert_eq!(f.c.tag_options(id, ""), ["carro", "casa", "ui-kit"]);
    assert_eq!(f.c.tag_options(id, "ca"), ["carro", "casa"]);
    assert_eq!(f.c.tag_options(id, "kit"), ["ui-kit"], "contém, e a tarefa já tem #kit");
}

#[test]
fn tag_options_strip_hash_and_spaces() {
    let (f, id) = project_with_tags(&["casa", "carro"]);
    assert_eq!(f.c.tag_options(id, " #CA "), ["carro", "casa"]);
}

#[test]
fn tag_options_are_capped_at_six() {
    let (f, id) = project_with_tags(&["t1", "t2", "t3", "t4", "t5", "t6", "t7", "t8"]);
    assert_eq!(f.c.tag_options(id, "").len(), 6);
}

#[test]
fn tag_key_down_then_enter_adds_the_highlighted_tag() {
    let (mut f, id) = project_with_tags(&["casa", "carro"]);
    f.c.tag_focus(id, true);
    assert_eq!(f.c.tag_key(id, "ca", Nav::Down), Ok(key(true, "ca")));
    assert_eq!(f.c.tag_highlighted(), Some(0));
    assert_eq!(f.c.tag_key(id, "ca", Nav::Enter), Ok(added()));
    assert_eq!(tag_names(&f, id), ["carro"]);
    assert!(!f.c.tag_list_open());
}

#[test]
fn tag_key_enter_without_highlight_adds_typed_text() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    assert_eq!(f.c.tag_key(id, "ca", Nav::Enter), Ok(added()));
    assert_eq!(tag_names(&f, id), ["ca"], "Enter usa o texto, não a sugestão");
}

#[test]
fn enter_on_empty_query_adds_nothing() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    assert_eq!(f.c.tag_key(id, "  ", Nav::Enter), Ok(key(true, "")));
    assert!(tag_names(&f, id).is_empty());
}

#[test]
fn tag_key_tab_completes() {
    let (mut f, id) = project_with_tags(&["casa", "carro"]);
    f.c.tag_focus(id, true);
    assert_eq!(f.c.tag_key(id, "cas", Nav::Tab), Ok(key(true, "casa")));
    assert_eq!(f.c.tag_key(id, "zz", Nav::Tab), Ok(key(false, "zz")));
    assert!(tag_names(&f, id).is_empty(), "Tab só completa o texto");
}

#[test]
fn tag_key_ignored_without_options() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    assert_eq!(f.c.tag_key(id, "zzz", Nav::Down), Ok(key(false, "zzz")), "↓ segue para o app");
    assert_eq!(f.c.tag_key(id, "zzz", Nav::Up), Ok(key(false, "zzz")));
}

#[test]
fn tag_key_escape_closes_then_is_ignored() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    assert!(f.c.tag_list_open());
    assert_eq!(f.c.tag_key(id, "", Nav::Escape), Ok(key(true, "")));
    assert!(!f.c.tag_list_open());
    assert_eq!(f.c.tag_key(id, "", Nav::Escape), Ok(key(false, "")), "o segundo Esc é do app");
}

#[test]
fn tag_pick_adds_and_closes() {
    let (mut f, id) = project_with_tags(&["casa", "carro"]);
    f.c.tag_focus(id, true);
    f.c.tag_key(id, "", Nav::Down).unwrap();
    f.c.tag_pick(id, "casa").unwrap();
    assert_eq!(tag_names(&f, id), ["casa"]);
    assert!(!f.c.tag_list_open());
    assert_eq!(f.c.tag_highlighted(), None);
}

#[test]
fn tag_list_closes_when_its_task_is_no_longer_open() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.toggle_expand(id);
    f.c.tag_focus(id, true);
    assert!(f.c.tag_list_open());
    f.c.drop_stale_tag_list();
    assert!(f.c.tag_list_open(), "a tarefa da lista continua aberta");
    let other = f.c.rows().into_iter().find(|r| r.id != id).unwrap().id;
    f.c.toggle_expand(other);
    f.c.drop_stale_tag_list();
    assert!(!f.c.tag_list_open());
    f.c.toggle_expand(id);
    f.c.drop_stale_tag_list();
    assert!(!f.c.tag_list_open(), "reabrir a tarefa não reabre a lista sem foco no campo");
}

#[test]
fn blur_from_another_field_does_not_close_the_list() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    f.c.tag_focus(id + 100, false);
    assert!(f.c.tag_list_open());
    f.c.tag_focus(id, false);
    assert!(!f.c.tag_list_open());
}

#[test]
fn tag_key_reports_whether_a_tag_was_added() {
    let (mut f, id) = project_with_tags(&["casa"]);
    f.c.tag_focus(id, true);
    assert!(!f.c.tag_key(id, "", Nav::Down).unwrap().added);
    assert!(!f.c.tag_key(id, "", Nav::Up).unwrap().added);
    assert!(!f.c.tag_key(id, "", Nav::Enter).unwrap().added, "Enter vazio não adiciona");
    assert!(f.c.tag_key(id, "novo", Nav::Enter).unwrap().added);
    f.c.tag_input();
    f.c.tag_key(id, "", Nav::Down).unwrap();
    assert!(f.c.tag_key(id, "", Nav::Enter).unwrap().added, "Pick adiciona");
}

#[test]
fn escape_closes_help_then_settings_then_task_state() {
    let mut f = at("2026-10-05 13:35");
    f.c.start_adding();
    f.c.open_settings();
    f.c.toggle_help();
    assert!(f.c.help_open);
    f.c.escape();
    assert!(!f.c.help_open);
    assert_eq!(f.c.page, Page::Settings);
    f.c.escape();
    assert_eq!(f.c.page, Page::Tasks);
    assert!(f.c.adding, "o Esc que saiu das configurações não mexe na lista");
    f.c.escape();
    assert!(!f.c.adding);
}

#[test]
fn selecting_a_view_leaves_settings() {
    let mut f = at("2026-10-05 13:35");
    f.c.open_settings();
    f.c.select_view(View::Inbox);
    assert_eq!(f.c.page, Page::Tasks);
}

#[test]
fn theme_and_update_toggle_are_persisted() {
    let mut f = at("2026-10-05 13:35");
    f.c.set_theme(ThemeMode::Dark);
    f.c.set_update_check(false);
    assert_eq!(f.c.store.settings().theme, ThemeMode::Dark);
    assert!(!f.c.store.settings().update_check);
    let sent = f.sent.borrow();
    assert!(sent.contains(&WriteOp::SetSetting { key: "theme", value: "dark".into() }));
    assert!(sent.contains(&WriteOp::SetSetting { key: "update_check", value: "0".into() }));
}
