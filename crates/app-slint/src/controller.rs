use std::collections::HashSet;

use chrono::{Days, NaiveDate, NaiveDateTime};
use ray_core::{matches_query, DomainError, Due, ProjectId, Store, TagId, Task, TaskId, View, WriteOp, PROJECT_COLORS};

use crate::present::{self, CalDay, Tone};

const INBOX_COLOR: &str = "#8E8E93";

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: TaskId,
    pub title: String,
    pub notes: String,
    pub done: bool,
    pub leaving: bool,
    pub pulse: bool,
    pub fresh: bool,
    pub group_header: String,
    pub group_sub: String,
    pub meta: String,
    pub tone: Tone,
    pub project_name: String,
    pub project_color: String,
    pub show_project: bool,
    pub has_project: bool,
    pub date_label: String,
    pub date_tone: Tone,
    pub time_label: String,
    pub time_tone: Tone,
    pub tags: Vec<(TagId, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavRow {
    pub view: View,
    pub label: String,
    pub color: String,
    pub count: usize,
    pub alert: bool,
}

impl NavRow {
    pub fn view_project(&self) -> Option<ProjectId> {
        match self.view {
            View::Project(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickDate {
    Today,
    Tomorrow,
    NextWeek,
    Clear,
}

pub struct Controller {
    pub store: Store,
    persist: Box<dyn FnMut(Vec<WriteOp>)>,
    pub view: View,
    pub expanded: Option<TaskId>,
    pub selected: Option<TaskId>,
    pub adding: bool,
    pub filter: String,
    pub filter_open: bool,
    pub show_done: bool,
    /// Tarefa recém-criada (a linha entra crescendo uma única vez).
    pub fresh: Option<TaskId>,
    pub popover_task: Option<TaskId>,
    pub popover_month: NaiveDate,
    lingering: HashSet<TaskId>,
    leaving: HashSet<TaskId>,
    pulsing: HashSet<TaskId>,
}

impl Controller {
    pub fn new(store: Store, persist: Box<dyn FnMut(Vec<WriteOp>)>) -> Self {
        let popover_month = present::first_of_month(store.today());
        Self {
            store,
            persist,
            view: View::Today,
            expanded: None,
            selected: None,
            adding: false,
            filter: String::new(),
            filter_open: false,
            show_done: false,
            fresh: None,
            popover_task: None,
            popover_month,
            lingering: HashSet::new(),
            leaving: HashSet::new(),
            pulsing: HashSet::new(),
        }
    }

    fn flush(&mut self) {
        let ops = self.store.take_ops();
        if !ops.is_empty() {
            (self.persist)(ops);
        }
    }

    // ---------- leitura ----------

    pub fn nav_rows(&self) -> Vec<NavRow> {
        let counts = self.store.counts();
        let mut rows = vec![
            NavRow { view: View::Today, label: "Hoje".into(), color: String::new(), count: counts.today, alert: counts.overdue },
            NavRow { view: View::Upcoming, label: "Próximos".into(), color: String::new(), count: counts.upcoming, alert: false },
            NavRow { view: View::Inbox, label: "Entrada".into(), color: String::new(), count: counts.inbox, alert: false },
        ];
        rows.extend(self.store.projects().iter().map(|p| NavRow {
            view: View::Project(p.id),
            label: p.name.clone(),
            color: p.color.clone(),
            count: counts.per_project.get(&p.id).copied().unwrap_or(0),
            alert: false,
        }));
        rows
    }

    pub fn selected_nav(&self) -> usize {
        self.nav_rows().iter().position(|r| r.view == self.view).unwrap_or(0)
    }

    pub fn title(&self) -> String {
        match self.view {
            View::Today => "Hoje".into(),
            View::Upcoming => "Próximos".into(),
            View::Inbox => "Entrada".into(),
            View::Project(id) => self.store.project(id).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }

    pub fn subtitle(&self) -> String {
        match self.view {
            View::Today => present::header_date(self.store.today()),
            View::Upcoming => "Próximos dias".into(),
            View::Inbox => "Capture agora, organize depois".into(),
            View::Project(id) => match self.store.counts().per_project.get(&id).copied().unwrap_or(0) {
                1 => "1 pendente".into(),
                n => format!("{n} pendentes"),
            },
        }
    }

    fn keep(&self) -> HashSet<TaskId> {
        self.lingering.union(&self.leaving).copied().collect()
    }

    pub fn rows(&self) -> Vec<Row> {
        let now = self.store.now();
        let today = now.date();
        let keep = self.keep();
        let mut rows = Vec::new();
        let mut last_date = None;
        for task in self.store.view(self.view, &keep).into_iter().filter(|t| matches_query(t, &self.filter)) {
            let mut group = (String::new(), String::new());
            if self.view == View::Upcoming {
                let date = task.due.expect("Próximos sempre tem data").date;
                if last_date != Some(date) {
                    last_date = Some(date);
                    group = present::group_header(date, today);
                }
            }
            rows.push(self.row(task, group, now));
        }
        if let (View::Project(id), true) = (self.view, self.show_done) {
            let done = self.store.completed_in_project(id).into_iter().filter(|t| !keep.contains(&t.id) && matches_query(t, &self.filter));
            for (i, task) in done.enumerate() {
                let group = if i == 0 { ("Concluídas".to_string(), String::new()) } else { Default::default() };
                rows.push(self.row(task, group, now));
            }
        }
        rows
    }

    fn row(&self, task: &Task, group: (String, String), now: NaiveDateTime) -> Row {
        let project = task.project_id.and_then(|id| self.store.project(id));
        let (meta, tone) = present::meta(task.due, now, self.view);
        let (time_label, time_tone) = present::time_label(task.due, now);
        let (date_label, date_tone) = match task.due {
            Some(d) => (present::date_label(d.date, now.date()), if d.date < now.date() { Tone::Late } else { Tone::Normal }),
            None => ("Sem data".to_string(), Tone::Normal),
        };
        Row {
            id: task.id,
            title: task.title.clone(),
            notes: task.notes.clone(),
            done: task.is_done(),
            leaving: self.leaving.contains(&task.id),
            pulse: self.pulsing.contains(&task.id),
            fresh: self.fresh == Some(task.id),
            group_header: group.0,
            group_sub: group.1,
            meta,
            tone,
            project_name: project.map(|p| p.name.clone()).unwrap_or_else(|| "Entrada".into()),
            project_color: project.map(|p| p.color.clone()).unwrap_or_else(|| INBOX_COLOR.into()),
            show_project: matches!(self.view, View::Today | View::Upcoming),
            has_project: project.is_some(),
            date_label,
            date_tone,
            time_label,
            time_tone,
            tags: task.tags.iter().filter_map(|id| self.store.tag(*id)).map(|t| (t.id, t.name.clone())).collect(),
        }
    }

    pub fn empty_text(&self) -> String {
        if self.adding || !self.rows().is_empty() {
            String::new()
        } else if !self.filter.trim().is_empty() {
            format!("Nada encontrado para “{}”.", self.filter.trim())
        } else {
            "Nada por aqui. Ctrl+N para capturar uma tarefa.".into()
        }
    }

    pub fn done_toggle_label(&self) -> String {
        let View::Project(id) = self.view else { return String::new() };
        match (self.store.completed_in_project(id).len(), self.show_done) {
            (0, _) => String::new(),
            (n, true) => format!("Ocultar concluídas ({n})"),
            (n, false) => format!("Mostrar concluídas ({n})"),
        }
    }

    /// Destinos de "mover para": Entrada + projetos. (id, nome, cor)
    pub fn move_targets(&self) -> Vec<(Option<ProjectId>, String, String)> {
        let mut targets = vec![(None, "Entrada".to_string(), INBOX_COLOR.to_string())];
        targets.extend(self.store.projects().iter().map(|p| (Some(p.id), p.name.clone(), p.color.clone())));
        targets
    }

    pub fn focused_task(&self) -> Option<TaskId> {
        self.expanded.or(self.selected)
    }

    pub fn project_task_count(&self, id: ProjectId) -> usize {
        self.store.task_count_in_project(id)
    }

    pub fn move_needs_confirm(&self, id: TaskId, to: Option<ProjectId>) -> bool {
        self.store.task(id).is_some_and(|t| t.project_id != to && !t.tags.is_empty())
    }

    pub fn tag_suggestion(&self, id: TaskId, prefix: &str) -> String {
        let prefix = prefix.trim().trim_start_matches('#').to_lowercase();
        let Some(task) = self.store.task(id) else { return String::new() };
        let Some(project_id) = task.project_id else { return String::new() };
        if prefix.is_empty() {
            return String::new();
        }
        self.store
            .project_tags(project_id)
            .into_iter()
            .filter(|t| !task.tags.contains(&t.id))
            .find(|t| t.name.to_lowercase().starts_with(&prefix))
            .map(|t| t.name.clone())
            .unwrap_or_default()
    }

    pub fn calendar(&self) -> (String, Vec<CalDay>) {
        let selected = self.popover_task.and_then(|id| self.store.task(id)).and_then(|t| t.due).map(|d| d.date);
        (present::month_title(self.popover_month), present::calendar(self.popover_month, self.store.today(), selected))
    }

    pub fn popover_time(&self) -> String {
        self.popover_task
            .and_then(|id| self.store.task(id))
            .and_then(|t| t.due)
            .and_then(|d| d.time)
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_default()
    }

    pub fn next_week_hint(&self) -> String {
        let today = self.store.today();
        present::short_date(present::next_monday(today), today)
    }

    // ---------- navegação ----------

    pub fn select_view(&mut self, view: View) {
        self.view = view;
        self.expanded = None;
        self.selected = None;
        self.adding = false;
        self.filter.clear();
        self.filter_open = false;
        self.show_done = false;
        self.popover_task = None;
    }

    pub fn toggle_expand(&mut self, id: TaskId) {
        self.expanded = if self.expanded == Some(id) { None } else { Some(id) };
        self.selected = Some(id);
        self.adding = false;
        self.popover_task = None;
    }

    pub fn move_selection(&mut self, delta: i32) {
        let ids: Vec<TaskId> = self.rows().into_iter().filter(|r| !r.leaving).map(|r| r.id).collect();
        if ids.is_empty() {
            self.selected = None;
            return;
        }
        let next = match self.selected.and_then(|s| ids.iter().position(|x| *x == s)) {
            Some(i) => (i as i32 + delta).clamp(0, ids.len() as i32 - 1) as usize,
            None if delta >= 0 => 0,
            None => ids.len() - 1,
        };
        self.selected = Some(ids[next]);
    }

    /// Esc fecha uma camada por vez: captura → popover → tarefa aberta → filtro → seleção.
    pub fn escape(&mut self) {
        if self.adding {
            self.adding = false;
        } else if self.popover_task.is_some() {
            self.popover_task = None;
        } else if self.expanded.is_some() {
            self.expanded = None;
        } else if self.filter_open {
            self.filter_open = false;
            self.filter.clear();
        } else {
            self.selected = None;
        }
    }

    pub fn toggle_filter(&mut self) {
        self.filter_open = !self.filter_open;
        if !self.filter_open {
            self.filter.clear();
        }
    }

    pub fn set_filter(&mut self, query: &str) {
        self.filter = query.to_string();
    }

    pub fn toggle_show_done(&mut self) {
        self.show_done = !self.show_done;
    }

    // ---------- tarefas ----------

    pub fn start_adding(&mut self) {
        self.adding = true;
        self.expanded = None;
        self.popover_task = None;
    }

    pub fn cancel_adding(&mut self) {
        self.adding = false;
    }

    pub fn commit_new(&mut self, title: &str) -> Result<TaskId, DomainError> {
        let id = self.store.create_task(self.view, title)?;
        self.fresh = Some(id);
        self.flush();
        Ok(id)
    }

    pub fn edit_text(&mut self, id: TaskId, title: &str, notes: &str) -> Result<(), DomainError> {
        self.store.update_text(id, title, notes)?;
        self.flush();
        Ok(())
    }

    /// Retorna `true` se ficou concluída (a linha fica visível até `start_leaving`/`finish_leaving`).
    pub fn toggle(&mut self, id: TaskId) -> Result<bool, DomainError> {
        let done = self.store.toggle_complete(id)?;
        if done {
            self.lingering.insert(id);
        } else {
            self.lingering.remove(&id);
            self.leaving.remove(&id);
        }
        self.flush();
        Ok(done)
    }

    /// 600 ms após concluir. `false` se a conclusão foi desfeita nesse meio tempo.
    pub fn start_leaving(&mut self, id: TaskId) -> bool {
        let still_done = self.lingering.contains(&id) && self.store.task(id).is_some_and(|t| t.is_done());
        if still_done {
            self.leaving.insert(id);
        } else {
            self.lingering.remove(&id);
        }
        still_done
    }

    pub fn finish_leaving(&mut self, id: TaskId) {
        self.lingering.remove(&id);
        self.leaving.remove(&id);
        if self.expanded == Some(id) {
            self.expanded = None;
        }
        if self.selected == Some(id) {
            self.selected = None;
        }
    }

    pub fn begin_delete(&mut self, id: TaskId) -> bool {
        if self.store.task(id).is_none() {
            return false;
        }
        self.leaving.insert(id);
        true
    }

    pub fn finish_delete(&mut self, id: TaskId) -> Result<(), DomainError> {
        self.finish_leaving(id);
        self.store.delete_task(id)?;
        self.flush();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let undone = self.store.undo();
        if undone {
            self.flush();
        }
        undone
    }

    pub fn set_date(&mut self, id: TaskId, date: Option<NaiveDate>) -> Result<(), DomainError> {
        let time = self.store.task(id).ok_or(DomainError::TaskNotFound(id))?.due.and_then(|d| d.time);
        self.store.set_due(id, date.map(|date| Due { date, time }))?;
        self.flush();
        Ok(())
    }

    pub fn quick_date(&mut self, id: TaskId, quick: QuickDate) -> Result<(), DomainError> {
        let today = self.store.today();
        let date = match quick {
            QuickDate::Today => Some(today),
            QuickDate::Tomorrow => Some(today + Days::new(1)),
            QuickDate::NextWeek => Some(present::next_monday(today)),
            QuickDate::Clear => None,
        };
        self.set_date(id, date)
    }

    /// Vazio limpa a hora. `Ok(false)` = texto inválido, a hora anterior é mantida.
    pub fn set_time(&mut self, id: TaskId, text: &str) -> Result<bool, DomainError> {
        let due = self.store.task(id).ok_or(DomainError::TaskNotFound(id))?.due;
        if text.trim().is_empty() {
            self.store.set_due(id, due.map(|d| Due { time: None, ..d }))?;
            self.flush();
            return Ok(true);
        }
        let Some(time) = present::parse_time(text) else { return Ok(false) };
        let date = due.map(|d| d.date).unwrap_or_else(|| self.store.today());
        self.store.set_due(id, Some(Due { date, time: Some(time) }))?;
        self.flush();
        Ok(true)
    }

    // ---------- tags e mover ----------

    pub fn add_tag_by_name(&mut self, id: TaskId, name: &str) -> Result<(), DomainError> {
        let task = self.store.task(id).ok_or(DomainError::TaskNotFound(id))?;
        let project_id = task.project_id.ok_or(DomainError::TagProjectMismatch)?;
        let tag = self.store.ensure_tag(project_id, name)?;
        self.store.add_tag(id, tag)?;
        self.flush();
        Ok(())
    }

    pub fn remove_tag(&mut self, id: TaskId, tag: TagId) -> Result<(), DomainError> {
        self.store.remove_tag(id, tag)?;
        self.flush();
        Ok(())
    }

    pub fn remove_last_tag(&mut self, id: TaskId) -> Result<(), DomainError> {
        let last = self.store.task(id).ok_or(DomainError::TaskNotFound(id))?.tags.last().copied();
        match last {
            Some(tag) => self.remove_tag(id, tag),
            None => Ok(()),
        }
    }

    pub fn rename_tag(&mut self, tag: TagId, name: &str) -> Result<(), DomainError> {
        self.store.rename_tag(tag, name)?;
        self.flush();
        Ok(())
    }

    pub fn delete_tag(&mut self, tag: TagId) -> Result<(), DomainError> {
        self.store.delete_tag(tag)?;
        self.flush();
        Ok(())
    }

    pub fn move_task(&mut self, id: TaskId, to: Option<ProjectId>) -> Result<(), DomainError> {
        self.store.move_task(id, to)?;
        self.flush();
        Ok(())
    }

    // ---------- projetos ----------

    pub fn create_project(&mut self, name: &str) -> Result<ProjectId, DomainError> {
        let id = self.store.create_project(name)?;
        self.flush();
        self.select_view(View::Project(id));
        Ok(id)
    }

    pub fn rename_project(&mut self, id: ProjectId, name: &str) -> Result<(), DomainError> {
        self.store.rename_project(id, name)?;
        self.flush();
        Ok(())
    }

    /// `index` na paleta `PROJECT_COLORS`.
    pub fn set_project_color(&mut self, id: ProjectId, index: usize) -> Result<(), DomainError> {
        self.store.set_project_color(id, PROJECT_COLORS[index % PROJECT_COLORS.len()])?;
        self.flush();
        Ok(())
    }

    pub fn delete_project(&mut self, id: ProjectId) -> Result<usize, DomainError> {
        let removed = self.store.delete_project(id)?;
        if self.view == View::Project(id) {
            self.select_view(View::Today);
        }
        self.flush();
        Ok(removed)
    }

    // ---------- popover de data ----------

    pub fn open_popover(&mut self, id: TaskId) {
        let date = self.store.task(id).and_then(|t| t.due).map(|d| d.date).unwrap_or_else(|| self.store.today());
        self.popover_month = present::first_of_month(date);
        self.popover_task = Some(id);
        self.expanded = Some(id);
        self.selected = Some(id);
    }

    pub fn close_popover(&mut self) {
        self.popover_task = None;
    }

    pub fn shift_popover_month(&mut self, delta: i32) {
        self.popover_month = present::shift_month(self.popover_month, delta);
    }

    // ---------- relógio ----------

    /// Chamado a cada minuto. Retorna as tarefas cuja hora chegou desde `previous` (para o pulso).
    pub fn tick(&mut self, previous: NaiveDateTime) -> Vec<TaskId> {
        let now = self.store.now();
        if previous.date() != now.date() {
            return Vec::new();
        }
        let due_now: Vec<TaskId> = self
            .store
            .view(View::Today, &HashSet::new())
            .into_iter()
            .filter(|t| {
                t.due.is_some_and(|d| d.date == now.date() && d.time.is_some_and(|tm| tm > previous.time() && tm <= now.time()))
            })
            .map(|t| t.id)
            .collect();
        self.pulsing.extend(due_now.iter().copied());
        due_now
    }

    pub fn clear_pulse(&mut self) {
        self.pulsing.clear();
    }
}
