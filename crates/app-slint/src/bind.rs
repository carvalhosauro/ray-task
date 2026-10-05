use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

use ray_core::writer::WriterEvent;
use ray_core::{DomainError, TaskId, View};
use slint::{Color, ComponentHandle, Model, ModelRc, Timer, TimerMode, VecModel};

use crate::controller::{Controller, NavRow, Row};
use crate::{Actions, AppWindow, CalCell, NavItem, Picker, ProjectChoice, TagChip, TaskItem};

pub const TOAST_UNDO: i32 = 0;
pub const TOAST_RETRY: i32 = 1;

/// Ação aguardando resposta de um diálogo (Task 12).
#[allow(dead_code)] // used in Task 12–14
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
    #[allow(dead_code)] // used in Task 12–14
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
    let timers = Vec::new();
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

/// Atualiza o modelo no lugar: linhas da mesma tarefa são reaproveitadas para as animações rodarem.
fn sync_rows(model: &VecModel<TaskItem>, rows: Vec<TaskItem>) {
    let mut i = 0;
    for row in rows {
        let found = (i..model.row_count()).find(|&j| model.row_data(j).is_some_and(|r| r.id == row.id));
        match found {
            Some(j) => {
                for _ in i..j {
                    model.remove(i);
                }
                model.set_row_data(i, row);
            }
            None => model.insert(i, row),
        }
        i += 1;
    }
    while model.row_count() > i {
        model.remove(i);
    }
}

pub(crate) fn refresh(s: &Shared) {
    let Some(ui) = s.ui.upgrade() else { return };
    let c = s.ctrl.borrow();
    ui.set_nav(ModelRc::new(VecModel::from(c.nav_rows().into_iter().map(nav_item).collect::<Vec<_>>())));
    ui.set_selected_nav(c.selected_nav() as i32);
    ui.set_view_title(c.title().into());
    ui.set_view_subtitle(c.subtitle().into());
    sync_rows(&s.tasks, c.rows().into_iter().map(task_item).collect());
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
    picker.set_next_week_hint(c.next_week_hint().into());
}

pub(crate) fn show_toast(s: &Shared, text: &str, action: &str, kind: i32) {
    let Some(ui) = s.ui.upgrade() else { return };
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
        if let Some(ui) = weak.upgrade() {
            ui.set_toast_visible(false);
        }
    });
}

fn hide_toast(s: &Shared) {
    if let Some(ui) = s.ui.upgrade() {
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
        });
    }
    {
        let s = s.clone();
        // Título vazio durante a digitação é ignorado (o Store recusa e mantém o anterior).
        actions.on_edit(move |id, title, notes| update(&s, |c| {
            let _ = c.edit_text(id as TaskId, &title, &notes);
        }));
    }
    {
        let s = s.clone();
        actions.on_delete(move |id| delete(&s, id as TaskId));
    }
    {
        let s = s.clone();
        actions.on_undo(move || {
            hide_toast(&s);
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
