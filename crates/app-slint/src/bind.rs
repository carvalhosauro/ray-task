use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime, Timelike};
use ray_core::writer::WriterEvent;
use ray_core::{DomainError, TaskId, View};
use slint::{Color, ComponentHandle, Model, ModelRc, Timer, TimerMode, VecModel};

use crate::controller::{Controller, NavRow, QuickDate, Row};
use crate::{Actions, AppWindow, CalCell, NavItem, Picker, ProjectChoice, TagChip, TaskItem};

pub const TOAST_UNDO: i32 = 0;
pub const TOAST_RETRY: i32 = 1;

/// Ação aguardando resposta de um diálogo (Task 12).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Pending {
    NewProject,
    RenameProject(ray_core::ProjectId),
    RenameTag(ray_core::TagId),
    DeleteProject(ray_core::ProjectId),
    DeleteTag(ray_core::TagId),
    MoveTask(TaskId, Option<ray_core::ProjectId>),
}

pub(crate) struct Shared {
    pub(crate) ui: slint::Weak<AppWindow>,
    pub(crate) ctrl: RefCell<Controller>,
    tasks: Rc<VecModel<TaskItem>>,
    pub(crate) pending: RefCell<Option<Pending>>,
    toast_timer: Timer,
    pub(crate) retry: Box<dyn Fn()>,
}

pub struct Binding {
    shared: Rc<Shared>,
    _timers: Vec<Rc<Timer>>,
}

impl Binding {
    pub fn controller(&self) -> Ref<'_, Controller> {
        self.shared.ctrl.borrow()
    }

    /// Define a hora de uma tarefa sem passar pela UI (usado em testes).
    pub fn set_time_for_test(&self, id: TaskId, text: &str) {
        update(&self.shared, |c| {
            let _ = c.set_time(id, text);
        });
    }
}

pub fn bind(ui: &AppWindow, ctrl: Controller, retry: Box<dyn Fn()>) -> Binding {
    let tasks = Rc::new(VecModel::<TaskItem>::default());
    ui.set_tasks(ModelRc::from(tasks.clone()));
    let shared = Rc::new(Shared {
        ui: ui.as_weak(),
        ctrl: RefCell::new(ctrl),
        tasks,
        pending: RefCell::new(None),
        toast_timer: Timer::default(),
        retry,
    });
    wire_tasks(ui, &shared);
    wire_projects(ui, &shared);
    wire_details(ui, &shared);
    wire_keyboard(ui, &shared);
    let timers = vec![start_clock(&shared)];
    refresh(&shared);
    Binding { shared, _timers: timers }
}

pub(crate) fn log_err<T>(result: Result<T, DomainError>, what: &str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(%error, "{what}");
            None
        }
    }
}

pub(crate) fn update(s: &Shared, f: impl FnOnce(&mut Controller)) {
    f(&mut s.ctrl.borrow_mut());
    refresh(s);
}

fn hex(s: &str) -> Color {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0x8E8E93);
    Color::from_rgb_u8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn nav_item(row: NavRow) -> NavItem {
    let (kind, project_id) = match row.view {
        View::Today => (0, -1),
        View::Upcoming => (1, -1),
        View::Inbox => (2, -1),
        View::Project(id) => (3, id as i32),
    };
    NavItem { kind, project_id, label: row.label.into(), color: hex(&row.color), count: row.count as i32, alert: row.alert }
}

fn task_item(r: Row) -> TaskItem {
    TaskItem {
        id: r.id as i32,
        title: r.title.into(),
        notes: r.notes.into(),
        done: r.done,
        leaving: r.leaving,
        pulse: r.pulse,
        fresh: r.fresh,
        group_header: r.group_header.into(),
        group_sub: r.group_sub.into(),
        meta: r.meta.into(),
        tone: r.tone.as_int(),
        project_name: r.project_name.into(),
        project_color: hex(&r.project_color),
        show_project: r.show_project,
        has_project: r.has_project,
        date_label: r.date_label.into(),
        date_tone: r.date_tone.as_int(),
        time_label: r.time_label.into(),
        time_tone: r.time_tone.as_int(),
        tags: ModelRc::new(VecModel::from(
            r.tags.into_iter().map(|(id, name)| TagChip { id: id as i32, name: name.into() }).collect::<Vec<_>>(),
        )),
    }
}

pub(crate) fn refresh(s: &Shared) {
    let Some(ui) = s.ui.upgrade() else { return };
    let c = s.ctrl.borrow();
    ui.set_nav(ModelRc::new(VecModel::from(c.nav_rows().into_iter().map(nav_item).collect::<Vec<_>>())));
    ui.set_selected_nav(c.selected_nav() as i32);
    ui.set_view_title(c.title().into());
    ui.set_view_subtitle(c.subtitle().into());
    // Linhas da mesma tarefa são reaproveitadas para as animações rodarem.
    gus_list_slint::sync(&s.tasks, c.rows().into_iter().map(task_item).collect(), |r| r.id);
    ui.set_expanded_id(c.expanded.map_or(-1, |id| id as i32));
    ui.set_selected_id(c.selected.map_or(-1, |id| id as i32));
    ui.set_adding(c.adding);
    ui.set_empty_text(c.empty_text().into());
    ui.set_done_toggle_label(c.done_toggle_label().into());
    ui.set_filter_visible(c.filter_open);

    let targets: Vec<ProjectChoice> = c
        .move_targets()
        .into_iter()
        .map(|(id, label, color)| ProjectChoice { id: id.map_or(-1, |x| x as i32), label: label.into(), color: hex(&color) })
        .collect();
    ui.global::<Actions>().set_move_targets(ModelRc::new(VecModel::from(targets)));

    let picker = ui.global::<Picker>();
    let (month_title, days) = c.calendar();
    picker.set_month_title(month_title.into());
    picker.set_cells(ModelRc::new(VecModel::from(
        days.into_iter()
            .map(|d| CalCell {
                day: d.day as i32,
                iso: d.date.map(|x| x.format("%Y-%m-%d").to_string()).unwrap_or_default().into(),
                today: d.today,
                selected: d.selected,
                past: d.past,
            })
            .collect::<Vec<_>>(),
    )));
    picker.set_time_text(c.popover_time().into());
    // A tarefa do popover saiu da visão (o Controller soltou o foco): fecha o popover também.
    if c.popover_task.is_none() && picker.get_open_for() != -1 {
        picker.set_open_for(-1);
        picker.set_close_request(picker.get_close_request() + 1);
    }
    picker.set_next_week_hint(c.next_week_hint().into());
}

pub(crate) fn show_toast(s: &Shared, text: &str, action: &str, kind: i32) {
    let Some(ui) = s.ui.upgrade() else { return };
    if kind == TOAST_UNDO && ui.get_toast_visible() && ui.get_toast_kind() == TOAST_RETRY {
        return; // a falha de gravação tem prioridade até resolver
    }
    ui.set_toast_text(text.into());
    ui.set_toast_action(action.into());
    ui.set_toast_kind(kind);
    ui.set_toast_visible(true);
    if kind == TOAST_RETRY {
        s.toast_timer.stop(); // erro de gravação fica visível até resolver
        return;
    }
    let weak = s.ui.clone();
    s.toast_timer.start(TimerMode::SingleShot, Duration::from_secs(5), move || {
        if let Some(ui) = weak.upgrade().filter(|ui| ui.get_toast_kind() == TOAST_UNDO) {
            ui.set_toast_visible(false);
        }
    });
}

/// Esconde só o toast de desfazer; o de falha de gravação fica até resolver.
fn hide_undo_toast(s: &Shared) {
    if let Some(ui) = s.ui.upgrade().filter(|ui| ui.get_toast_kind() == TOAST_UNDO) {
        ui.set_toast_visible(false);
    }
}

/// Eventos da thread de escrita (chamado no event loop pelo `main`).
pub fn show_writer_event(ui: &AppWindow, event: &WriterEvent) {
    match event {
        WriterEvent::Failed { .. } => {
            ui.set_toast_text("Falha ao salvar".into());
            ui.set_toast_action("Tentar de novo".into());
            ui.set_toast_kind(TOAST_RETRY);
            ui.set_toast_visible(true);
        }
        WriterEvent::Recovered => {
            if ui.get_toast_kind() == TOAST_RETRY {
                ui.set_toast_visible(false);
            }
        }
    }
}

pub(crate) fn toggle(s: &Rc<Shared>, id: TaskId) {
    let done = log_err(s.ctrl.borrow_mut().toggle(id), "concluir tarefa");
    refresh(s);
    if done != Some(true) {
        return;
    }
    show_toast(s, "Tarefa concluída", "Desfazer", TOAST_UNDO);
    let s1 = s.clone();
    Timer::single_shot(Duration::from_millis(600), move || {
        let leaving = s1.ctrl.borrow_mut().start_leaving(id);
        refresh(&s1);
        if leaving {
            let s2 = s1.clone();
            Timer::single_shot(Duration::from_millis(220), move || update(&s2, |c| c.finish_leaving(id)));
        }
    });
}

pub(crate) fn delete(s: &Rc<Shared>, id: TaskId) {
    if !s.ctrl.borrow_mut().begin_delete(id) {
        return;
    }
    refresh(s);
    let s1 = s.clone();
    Timer::single_shot(Duration::from_millis(220), move || {
        let deleted = log_err(s1.ctrl.borrow_mut().finish_delete(id), "apagar tarefa").is_some();
        refresh(&s1);
        if deleted {
            show_toast(&s1, "Tarefa apagada", "Desfazer", TOAST_UNDO);
        }
    });
}

/// Crossfade: some (60 ms), troca o conteúdo, reaparece (60 ms). A seleção da sidebar desliza na hora.
pub(crate) fn switch_view(s: &Rc<Shared>, view: View, nav_index: usize) {
    let Some(ui) = s.ui.upgrade() else { return };
    if s.ctrl.borrow().view == view {
        return;
    }
    ui.set_selected_nav(nav_index as i32);
    ui.set_content_faded(true);
    let s1 = s.clone();
    Timer::single_shot(Duration::from_millis(60), move || {
        update(&s1, |c| c.select_view(view));
        // Pedido de data pendente da visão anterior não pode abrir o popover mais tarde.
        close_picker(&s1);
        if let Some(ui) = s1.ui.upgrade() {
            ui.set_content_faded(false);
            ui.invoke_focus_root();
        }
    });
}

fn wire_tasks(ui: &AppWindow, s: &Rc<Shared>) {
    let actions = ui.global::<Actions>();
    {
        let s = s.clone();
        actions.on_select_nav(move |index| {
            let view = s.ctrl.borrow().nav_rows().get(index as usize).map(|r| r.view);
            if let Some(view) = view {
                switch_view(&s, view, index as usize);
            }
        });
    }
    {
        let s = s.clone();
        actions.on_toggle(move |id| toggle(&s, id as TaskId));
    }
    {
        let s = s.clone();
        actions.on_expand(move |id| update(&s, |c| c.toggle_expand(id as TaskId)));
    }
    {
        let s = s.clone();
        actions.on_new_task(move || update(&s, |c| c.start_adding()));
    }
    {
        let s = s.clone();
        actions.on_commit_new(move |title| {
            update(&s, |c| {
                log_err(c.commit_new(&title), "criar tarefa");
            });
            s.ctrl.borrow_mut().fresh = None; // só a primeira renderização cresce
                                              // Captura seguida: o campo "Nova tarefa" vai na última linha, que desce a cada Enter.
                                              // A lista é virtual, então rola até ela para o campo continuar existindo (e com foco).
            let adding = s.ctrl.borrow().adding;
            if let (true, Some(ui), Some(last)) = (adding, s.ui.upgrade(), s.tasks.row_count().checked_sub(1)) {
                reveal_index(&ui, last);
            }
        });
    }
    {
        let s = s.clone();
        // Título vazio durante a digitação é ignorado (o Store recusa e mantém o anterior).
        actions.on_edit(move |id, title, notes| {
            update(&s, |c| {
                let _ = c.edit_text(id as TaskId, &title, &notes);
            })
        });
    }
    {
        let s = s.clone();
        actions.on_delete(move |id| delete(&s, id as TaskId));
    }
    {
        let s = s.clone();
        actions.on_undo(move || {
            hide_undo_toast(&s);
            update(&s, |c| {
                c.undo();
            });
        });
    }
    {
        let s = s.clone();
        actions.on_toast_action(move || {
            let Some(ui) = s.ui.upgrade() else { return };
            ui.set_toast_visible(false);
            if ui.get_toast_kind() == TOAST_RETRY {
                (s.retry)();
            } else {
                update(&s, |c| {
                    c.undo();
                });
            }
        });
    }
    {
        let s = s.clone();
        actions.on_toggle_show_done(move || update(&s, |c| c.toggle_show_done()));
    }
}

pub(crate) fn dialog_open(s: &Shared) -> bool {
    s.pending.borrow().is_some()
}

fn open_prompt(s: &Shared, title: &str, text: &str, pending: Pending) {
    let Some(ui) = s.ui.upgrade() else { return };
    *s.pending.borrow_mut() = Some(pending);
    ui.set_confirm_visible(false);
    ui.set_prompt_title(title.into());
    ui.set_prompt_text(text.into());
    ui.set_prompt_error("".into());
    ui.set_prompt_visible(true);
}

pub(crate) fn open_confirm(s: &Shared, title: &str, message: &str, button: &str, pending: Pending) {
    let Some(ui) = s.ui.upgrade() else { return };
    *s.pending.borrow_mut() = Some(pending);
    ui.set_prompt_visible(false);
    ui.set_confirm_title(title.into());
    ui.set_confirm_message(message.into());
    ui.set_confirm_button(button.into());
    ui.set_confirm_visible(true);
}

pub(crate) fn close_dialogs(s: &Shared) {
    *s.pending.borrow_mut() = None;
    if let Some(ui) = s.ui.upgrade() {
        ui.set_prompt_visible(false);
        ui.set_confirm_visible(false);
        ui.invoke_focus_root();
    }
}

fn tasks_phrase(n: usize) -> String {
    match n {
        0 => "O projeto está vazio.".into(),
        1 => "1 tarefa será apagada junto. Isso não pode ser desfeito.".into(),
        n => format!("{n} tarefas serão apagadas junto. Isso não pode ser desfeito."),
    }
}

fn wire_projects(ui: &AppWindow, s: &Rc<Shared>) {
    let actions = ui.global::<Actions>();
    {
        let s = s.clone();
        actions.on_new_project(move || open_prompt(&s, "Novo projeto", "", Pending::NewProject));
    }
    {
        let s = s.clone();
        actions.on_begin_rename_project(move |id| {
            let name = s.ctrl.borrow().store.project(id as i64).map(|p| p.name.clone());
            if let Some(name) = name {
                open_prompt(&s, "Renomear projeto", &name, Pending::RenameProject(id as i64));
            }
        });
    }
    {
        let s = s.clone();
        actions.on_set_project_color(move |id, index| {
            update(&s, |c| {
                log_err(c.set_project_color(id as i64, index as usize), "cor do projeto");
            })
        });
    }
    {
        let s = s.clone();
        actions.on_request_delete_project(move |id| {
            let info = {
                let c = s.ctrl.borrow();
                c.store.project(id as i64).map(|p| (p.name.clone(), c.project_task_count(id as i64)))
            };
            if let Some((name, count)) = info {
                open_confirm(&s, &format!("Apagar “{name}”?"), &tasks_phrase(count), "Apagar", Pending::DeleteProject(id as i64));
            }
        });
    }
    {
        let s = s.clone();
        actions.on_begin_rename_tag(move |id| {
            let name = s.ctrl.borrow().store.tag(id as i64).map(|t| t.name.clone());
            if let Some(name) = name {
                open_prompt(&s, "Renomear tag", &name, Pending::RenameTag(id as i64));
            }
        });
    }
    {
        let s = s.clone();
        actions.on_request_delete_tag(move |id| {
            let name = s.ctrl.borrow().store.tag(id as i64).map(|t| t.name.clone());
            if let Some(name) = name {
                open_confirm(
                    &s,
                    &format!("Apagar a tag “{name}”?"),
                    "Ela será removida de todas as tarefas do projeto.",
                    "Apagar",
                    Pending::DeleteTag(id as i64),
                );
            }
        });
    }
    {
        let s = s.clone();
        actions.on_prompt_accept(move |text| {
            let Some(pending) = *s.pending.borrow() else { return };
            let result = {
                let mut c = s.ctrl.borrow_mut();
                match pending {
                    Pending::NewProject => c.create_project(&text).map(|_| ()),
                    Pending::RenameProject(id) => c.rename_project(id, &text),
                    Pending::RenameTag(id) => c.rename_tag(id, &text),
                    _ => return,
                }
            };
            match result {
                Ok(()) => {
                    close_dialogs(&s);
                    refresh(&s);
                }
                Err(error) => {
                    if let Some(ui) = s.ui.upgrade() {
                        ui.set_prompt_error(error.to_string().into());
                    }
                }
            }
        });
    }
    {
        let s = s.clone();
        actions.on_confirm_accept(move || {
            let Some(pending) = s.pending.borrow_mut().take() else { return };
            {
                let mut c = s.ctrl.borrow_mut();
                match pending {
                    Pending::DeleteProject(id) => {
                        log_err(c.delete_project(id), "apagar projeto");
                    }
                    Pending::DeleteTag(id) => {
                        log_err(c.delete_tag(id), "apagar tag");
                    }
                    Pending::MoveTask(task, to) => {
                        log_err(c.move_task(task, to), "mover tarefa");
                    }
                    _ => {}
                }
            }
            close_dialogs(&s);
            refresh(&s);
        });
    }
    {
        let s = s.clone();
        actions.on_dialog_cancel(move || close_dialogs(&s));
    }
}

pub(crate) fn open_picker(s: &Rc<Shared>, id: TaskId) {
    update(s, |c| c.open_popover(id));
    if let Some(ui) = s.ui.upgrade() {
        let picker = ui.global::<Picker>();
        picker.set_open_for(id as i32);
        picker.set_request(picker.get_request() + 1);
    }
}

fn close_picker(s: &Shared) {
    s.ctrl.borrow_mut().close_popover();
    if let Some(ui) = s.ui.upgrade() {
        let picker = ui.global::<Picker>();
        picker.set_open_for(-1);
        picker.set_close_request(picker.get_close_request() + 1);
        // O foco estava no campo de hora do popover: volta para os atalhos globais.
        ui.invoke_focus_root();
    }
}

/// Pede à lista que role até a tarefa. A lista é virtual (a linha pode nem existir ainda):
/// o Slint estima a posição e depois corrige pelo layout real.
fn reveal(s: &Shared, id: TaskId) {
    let Some(ui) = s.ui.upgrade() else { return };
    let Some(index) = (0..s.tasks.row_count()).find(|&i| s.tasks.row_data(i).is_some_and(|r| r.id as TaskId == id)) else {
        return;
    };
    reveal_index(&ui, index);
}

fn reveal_index(ui: &AppWindow, index: usize) {
    ui.set_reveal_index(index as i32);
    ui.set_reveal_request(ui.get_reveal_request() + 1);
}

pub(crate) fn focus_tag_input(s: &Shared, id: TaskId) {
    if let Some(ui) = s.ui.upgrade() {
        let actions = ui.global::<Actions>();
        actions.set_focus_tag_task(id as i32);
        actions.set_focus_tag_request(actions.get_focus_tag_request() + 1);
    }
}

/// Foco no título da tarefa aberta (atendido pelo próprio campo quando ele existir).
fn focus_title(s: &Shared, id: TaskId) {
    if let Some(ui) = s.ui.upgrade() {
        let actions = ui.global::<Actions>();
        actions.set_focus_title_task(id as i32);
        actions.set_focus_title_request(actions.get_focus_title_request() + 1);
    }
}

fn request_move(s: &Rc<Shared>, id: TaskId, to: Option<ray_core::ProjectId>) {
    if s.ctrl.borrow().move_needs_confirm(id, to) {
        open_confirm(
            s,
            "Mover tarefa?",
            "As tags desta tarefa serão removidas, porque cada projeto tem as suas.",
            "Mover",
            Pending::MoveTask(id, to),
        );
    } else {
        update(s, |c| {
            log_err(c.move_task(id, to), "mover tarefa");
        });
    }
}

fn wire_details(ui: &AppWindow, s: &Rc<Shared>) {
    let picker = ui.global::<Picker>();
    {
        let s = s.clone();
        picker.on_open(move |id| open_picker(&s, id as TaskId));
    }
    {
        let s = s.clone();
        picker.on_shift_month(move |delta| update(&s, |c| c.shift_popover_month(delta)));
    }
    {
        let s = s.clone();
        picker.on_pick(move |id, iso| {
            let date = NaiveDate::parse_from_str(&iso, "%Y-%m-%d").ok();
            if date.is_some() {
                log_err(s.ctrl.borrow_mut().set_date(id as TaskId, date), "escolher data");
            }
            close_picker(&s);
            refresh(&s);
        });
    }
    {
        let s = s.clone();
        picker.on_quick(move |id, kind| {
            let quick = match kind {
                0 => QuickDate::Today,
                1 => QuickDate::Tomorrow,
                2 => QuickDate::NextWeek,
                _ => QuickDate::Clear,
            };
            log_err(s.ctrl.borrow_mut().quick_date(id as TaskId, quick), "data rápida");
            close_picker(&s);
            refresh(&s);
        });
    }
    {
        let s = s.clone();
        picker.on_set_time(move |id, text| {
            update(&s, |c| {
                if log_err(c.set_time(id as TaskId, &text), "hora") == Some(false) {
                    tracing::info!(%text, "hora inválida ignorada");
                }
            })
        });
    }

    let actions = ui.global::<Actions>();
    {
        let s = s.clone();
        actions.on_add_tag(move |id, name| {
            update(&s, |c| {
                log_err(c.add_tag_by_name(id as TaskId, &name), "adicionar tag");
            })
        });
    }
    {
        let s = s.clone();
        actions.on_remove_tag(move |id, tag| {
            update(&s, |c| {
                log_err(c.remove_tag(id as TaskId, tag as i64), "remover tag");
            })
        });
    }
    {
        let s = s.clone();
        actions.on_remove_last_tag(move |id| {
            update(&s, |c| {
                log_err(c.remove_last_tag(id as TaskId), "remover última tag");
            })
        });
    }
    {
        let s = s.clone();
        actions.on_tag_suggest(move |id, prefix| s.ctrl.borrow().tag_suggestion(id as TaskId, &prefix).into());
    }
    {
        let s = s.clone();
        actions.on_request_move(move |id, to| request_move(&s, id as TaskId, (to >= 0).then_some(to as i64)));
    }
}

fn wire_keyboard(ui: &AppWindow, s: &Rc<Shared>) {
    let actions = ui.global::<Actions>();
    {
        let s = s.clone();
        actions.on_escape(move || {
            if dialog_open(&s) {
                close_dialogs(&s);
                return;
            }
            let popover_was_open = s.ctrl.borrow().popover_task.is_some();
            update(&s, |c| c.escape());
            if popover_was_open {
                close_picker(&s);
            }
            if let Some(ui) = s.ui.upgrade() {
                ui.invoke_focus_root();
            }
        });
    }
    {
        let s = s.clone();
        actions.on_move_selection(move |delta| {
            update(&s, |c| c.move_selection(delta));
            let selected = s.ctrl.borrow().selected;
            if let Some(id) = selected {
                reveal(&s, id);
            }
        });
    }
    {
        let s = s.clone();
        actions.on_expand_selected(move || {
            update(&s, |c| {
                if let Some(id) = c.selected {
                    c.toggle_expand(id);
                }
            });
            let expanded = s.ctrl.borrow().expanded;
            if let Some(id) = expanded {
                focus_title(&s, id);
            }
        });
    }
    {
        let s = s.clone();
        actions.on_toggle_selected(move || {
            let target = s.ctrl.borrow().focused_task();
            if let Some(id) = target {
                toggle(&s, id);
            }
        });
    }
    {
        let s = s.clone();
        actions.on_delete_selected(move || {
            let target = s.ctrl.borrow().focused_task();
            if let Some(id) = target {
                delete(&s, id);
            }
        });
    }
    {
        let s = s.clone();
        // Ctrl+D: abre a tarefa, rola até ela e, no frame seguinte, pede o popover
        // (se os detalhes nascerem depois, eles atendem o pedido pendente).
        actions.on_date_selected(move || {
            let target = s.ctrl.borrow().focused_task();
            let Some(id) = target else { return };
            update(&s, |c| c.expanded = Some(id));
            reveal(&s, id);
            let s1 = s.clone();
            Timer::single_shot(Duration::from_millis(16), move || open_picker(&s1, id));
        });
    }
    {
        let s = s.clone();
        actions.on_tag_selected(move || {
            let target = s.ctrl.borrow().focused_task();
            let Some(id) = target else { return };
            update(&s, |c| c.expanded = Some(id));
            reveal(&s, id);
            let s1 = s.clone();
            Timer::single_shot(Duration::from_millis(16), move || focus_tag_input(&s1, id));
        });
    }
    {
        let s = s.clone();
        actions.on_toggle_filter(move || {
            update(&s, |c| c.toggle_filter());
            if let Some(ui) = s.ui.upgrade() {
                if !ui.get_filter_visible() {
                    ui.invoke_focus_root();
                }
            }
        });
    }
    {
        let s = s.clone();
        actions.on_filter_changed(move |text| update(&s, |c| c.set_filter(&text)));
    }
}

/// Timer alinhado à virada do minuto: recalcula "em 25 min"/"há 10 min", vira o dia e dispara o pulso.
fn start_clock(s: &Rc<Shared>) -> Rc<Timer> {
    let timer = Rc::new(Timer::default());
    let now = s.ctrl.borrow().store.now();
    let first = Duration::from_secs(60 - u64::from(now.second()));
    let last = Rc::new(Cell::new(now));
    let (s1, timer1) = (s.clone(), timer.clone());
    Timer::single_shot(first, move || {
        minute_tick(&s1, &last);
        let (s2, last2) = (s1.clone(), last.clone());
        timer1.start(TimerMode::Repeated, Duration::from_secs(60), move || minute_tick(&s2, &last2));
    });
    timer
}

fn minute_tick(s: &Rc<Shared>, last: &Cell<NaiveDateTime>) {
    let now = s.ctrl.borrow().store.now();
    let previous = last.replace(now);
    let pulsed = s.ctrl.borrow_mut().tick(previous);
    refresh(s);
    if !pulsed.is_empty() {
        let s1 = s.clone();
        Timer::single_shot(Duration::from_millis(700), move || update(&s1, |c| c.clear_pulse()));
    }
}
