use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, Utc};

use crate::clock::Clock;
use crate::error::DomainError;
use crate::model::*;
use crate::ops::WriteOp;

pub const UNDO_LIMIT: usize = 20;

#[derive(Debug, Clone)]
pub(crate) enum UndoEntry {
    Deleted(Task),
    Completed { id: TaskId, previous: Option<DateTime<Utc>> },
    Moved { id: TaskId, project_id: Option<ProjectId>, tags: Vec<TagId> },
}

pub struct Store {
    pub(crate) projects: Vec<Project>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) tasks: HashMap<TaskId, Task>,
    pub(crate) clock: Box<dyn Clock>,
    next_project: ProjectId,
    next_tag: TagId,
    next_task: TaskId,
    next_sort: i64,
    pub(crate) ops: Vec<WriteOp>,
    pub(crate) undo: Vec<UndoEntry>,
}

impl Store {
    pub fn new(snapshot: Snapshot, clock: Box<dyn Clock>) -> Self {
        let mut projects = snapshot.projects;
        projects.sort_by_key(|p| (p.sort_order, p.id));
        let next_project = projects.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let next_tag = snapshot.tags.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let next_task = snapshot.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let next_sort = snapshot.tasks.iter().map(|t| t.sort_order).max().unwrap_or(0) + 1;
        Self {
            projects,
            tags: snapshot.tags,
            tasks: snapshot.tasks.into_iter().map(|t| (t.id, t)).collect(),
            clock,
            next_project,
            next_tag,
            next_task,
            next_sort,
            ops: Vec::new(),
            undo: Vec::new(),
        }
    }

    // ---------- leitura ----------

    pub fn now(&self) -> NaiveDateTime {
        self.clock.now_local()
    }

    pub fn today(&self) -> NaiveDate {
        self.clock.today()
    }

    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    pub fn project(&self, id: ProjectId) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.iter().find(|t| t.id == id)
    }

    pub fn project_tags(&self, project_id: ProjectId) -> Vec<&Tag> {
        let mut tags: Vec<&Tag> = self.tags.iter().filter(|t| t.project_id == project_id).collect();
        tags.sort_by_key(|t| t.name.to_lowercase());
        tags
    }

    pub fn task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn task_count_in_project(&self, project_id: ProjectId) -> usize {
        self.tasks.values().filter(|t| t.project_id == Some(project_id)).count()
    }

    pub fn take_ops(&mut self) -> Vec<WriteOp> {
        std::mem::take(&mut self.ops)
    }

    // ---------- projetos ----------

    pub fn create_project(&mut self, name: &str) -> Result<ProjectId, DomainError> {
        let name = normalize_name(name).ok_or(DomainError::EmptyName)?;
        let id = self.next_project;
        self.next_project += 1;
        let project = Project {
            id,
            name,
            color: PROJECT_COLORS[self.projects.len() % PROJECT_COLORS.len()].to_string(),
            sort_order: self.projects.iter().map(|p| p.sort_order).max().map_or(0, |m| m + 1),
            created_at: self.clock.now_utc(),
        };
        self.ops.push(WriteOp::UpsertProject(project.clone()));
        self.projects.push(project);
        Ok(id)
    }

    pub fn rename_project(&mut self, id: ProjectId, name: &str) -> Result<(), DomainError> {
        let name = normalize_name(name).ok_or(DomainError::EmptyName)?;
        let project = self.projects.iter_mut().find(|p| p.id == id).ok_or(DomainError::ProjectNotFound(id))?;
        project.name = name;
        let snapshot = project.clone();
        self.ops.push(WriteOp::UpsertProject(snapshot));
        Ok(())
    }

    pub fn set_project_color(&mut self, id: ProjectId, color: &str) -> Result<(), DomainError> {
        let project = self.projects.iter_mut().find(|p| p.id == id).ok_or(DomainError::ProjectNotFound(id))?;
        project.color = color.to_string();
        let snapshot = project.clone();
        self.ops.push(WriteOp::UpsertProject(snapshot));
        Ok(())
    }

    /// Apaga o projeto, suas tarefas e tags. Retorna quantas tarefas foram apagadas.
    pub fn delete_project(&mut self, id: ProjectId) -> Result<usize, DomainError> {
        let pos = self.projects.iter().position(|p| p.id == id).ok_or(DomainError::ProjectNotFound(id))?;
        self.projects.remove(pos);
        let before = self.tasks.len();
        self.tasks.retain(|_, t| t.project_id != Some(id));
        let removed = before - self.tasks.len();
        self.tags.retain(|t| t.project_id != id);
        let tasks = &self.tasks;
        self.undo.retain(|entry| match entry {
            UndoEntry::Deleted(t) => t.project_id != Some(id),
            UndoEntry::Completed { id: task_id, .. } => tasks.contains_key(task_id),
            UndoEntry::Moved { id: task_id, project_id, .. } => tasks.contains_key(task_id) && *project_id != Some(id),
        });
        self.ops.push(WriteOp::DeleteProject(id));
        Ok(removed)
    }

    // ---------- tarefas ----------

    pub fn create_task(&mut self, context: View, title: &str) -> Result<TaskId, DomainError> {
        let title = normalize_name(title).ok_or(DomainError::EmptyTitle)?;
        let today = self.today();
        let (project_id, due) = match context {
            View::Today => (None, Some(Due { date: today, time: None })),
            View::Upcoming => (None, Some(Due { date: today + Days::new(1), time: None })),
            View::Inbox => (None, None),
            View::Project(pid) => {
                if self.project(pid).is_none() {
                    return Err(DomainError::ProjectNotFound(pid));
                }
                (Some(pid), None)
            }
        };
        let id = self.next_task;
        self.next_task += 1;
        let sort_order = self.next_sort;
        self.next_sort += 1;
        let now = self.clock.now_utc();
        let task = Task { id, project_id, title, notes: String::new(), due, completed_at: None, sort_order, created_at: now, updated_at: now, tags: Vec::new() };
        self.ops.push(WriteOp::UpsertTask(task.clone()));
        self.tasks.insert(id, task);
        Ok(id)
    }

    pub fn update_text(&mut self, id: TaskId, title: &str, notes: &str) -> Result<(), DomainError> {
        let title = normalize_name(title).ok_or(DomainError::EmptyTitle)?;
        let task = self.task_mut(id)?;
        if task.title == title && task.notes == notes {
            return Ok(());
        }
        task.title = title;
        task.notes = notes.to_string();
        self.save_task(id);
        Ok(())
    }

    pub fn set_due(&mut self, id: TaskId, due: Option<Due>) -> Result<(), DomainError> {
        let task = self.task_mut(id)?;
        if task.due == due {
            return Ok(());
        }
        task.due = due;
        self.save_task(id);
        Ok(())
    }

    /// Retorna `true` se a tarefa ficou concluída.
    pub fn toggle_complete(&mut self, id: TaskId) -> Result<bool, DomainError> {
        let now = self.clock.now_utc();
        let task = self.task_mut(id)?;
        let previous = task.completed_at;
        task.completed_at = if previous.is_some() { None } else { Some(now) };
        let done = task.completed_at.is_some();
        self.remember(UndoEntry::Completed { id, previous });
        self.save_task(id);
        Ok(done)
    }

    /// Move para outro projeto (`None` = Entrada). As tags são removidas.
    pub fn move_task(&mut self, id: TaskId, to: Option<ProjectId>) -> Result<(), DomainError> {
        if let Some(pid) = to {
            if self.project(pid).is_none() {
                return Err(DomainError::ProjectNotFound(pid));
            }
        }
        let task = self.task_mut(id)?;
        if task.project_id == to {
            return Ok(());
        }
        let entry = UndoEntry::Moved { id, project_id: task.project_id, tags: std::mem::take(&mut task.tags) };
        task.project_id = to;
        self.remember(entry);
        self.save_task(id);
        Ok(())
    }

    pub fn delete_task(&mut self, id: TaskId) -> Result<(), DomainError> {
        let task = self.tasks.remove(&id).ok_or(DomainError::TaskNotFound(id))?;
        self.remember(UndoEntry::Deleted(task));
        self.ops.push(WriteOp::DeleteTask(id));
        Ok(())
    }

    // ---------- tags ----------

    /// Devolve a tag do projeto com esse nome (sem diferenciar maiúsculas) ou cria uma nova.
    pub fn ensure_tag(&mut self, project_id: ProjectId, name: &str) -> Result<TagId, DomainError> {
        let name = normalize_tag_name(name).ok_or(DomainError::EmptyName)?;
        if self.project(project_id).is_none() {
            return Err(DomainError::ProjectNotFound(project_id));
        }
        let key = name.to_lowercase();
        if let Some(existing) = self.tags.iter().find(|t| t.project_id == project_id && t.name.to_lowercase() == key) {
            return Ok(existing.id);
        }
        let id = self.next_tag;
        self.next_tag += 1;
        let tag = Tag { id, project_id, name, color: TAG_COLOR.to_string() };
        self.ops.push(WriteOp::UpsertTag(tag.clone()));
        self.tags.push(tag);
        Ok(id)
    }

    pub fn add_tag(&mut self, task_id: TaskId, tag_id: TagId) -> Result<(), DomainError> {
        let tag_project = self.tag(tag_id).ok_or(DomainError::TagNotFound(tag_id))?.project_id;
        let task = self.task_mut(task_id)?;
        if task.project_id != Some(tag_project) {
            return Err(DomainError::TagProjectMismatch);
        }
        if task.tags.contains(&tag_id) {
            return Ok(());
        }
        task.tags.push(tag_id);
        self.save_task(task_id);
        Ok(())
    }

    pub fn remove_tag(&mut self, task_id: TaskId, tag_id: TagId) -> Result<(), DomainError> {
        let task = self.task_mut(task_id)?;
        let before = task.tags.len();
        task.tags.retain(|t| *t != tag_id);
        if task.tags.len() != before {
            self.save_task(task_id);
        }
        Ok(())
    }

    pub fn rename_tag(&mut self, tag_id: TagId, name: &str) -> Result<(), DomainError> {
        let name = normalize_tag_name(name).ok_or(DomainError::EmptyName)?;
        let project_id = self.tag(tag_id).ok_or(DomainError::TagNotFound(tag_id))?.project_id;
        let key = name.to_lowercase();
        if self.tags.iter().any(|t| t.project_id == project_id && t.id != tag_id && t.name.to_lowercase() == key) {
            return Err(DomainError::DuplicateTag);
        }
        let tag = self.tags.iter_mut().find(|t| t.id == tag_id).expect("tag checada acima");
        tag.name = name;
        let snapshot = tag.clone();
        self.ops.push(WriteOp::UpsertTag(snapshot));
        Ok(())
    }

    pub fn delete_tag(&mut self, tag_id: TagId) -> Result<(), DomainError> {
        let pos = self.tags.iter().position(|t| t.id == tag_id).ok_or(DomainError::TagNotFound(tag_id))?;
        self.tags.remove(pos);
        for task in self.tasks.values_mut() {
            task.tags.retain(|t| *t != tag_id);
        }
        for entry in self.undo.iter_mut() {
            match entry {
                UndoEntry::Deleted(task) => task.tags.retain(|t| *t != tag_id),
                UndoEntry::Moved { tags, .. } => tags.retain(|t| *t != tag_id),
                UndoEntry::Completed { .. } => {}
            }
        }
        // O banco remove as linhas de task_tags por cascade.
        self.ops.push(WriteOp::DeleteTag(tag_id));
        Ok(())
    }

    // ---------- desfazer ----------

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Desfaz a ação mais recente que ainda faz sentido. Entradas que não podem mais
    /// ser aplicadas (tarefa ou projeto sumiram) são descartadas.
    pub fn undo(&mut self) -> bool {
        while let Some(entry) = self.undo.pop() {
            match entry {
                UndoEntry::Deleted(task) => {
                    if task.project_id.is_some_and(|p| self.project(p).is_none()) {
                        continue;
                    }
                    let id = task.id;
                    self.tasks.insert(id, task);
                    self.save_task(id);
                    return true;
                }
                UndoEntry::Completed { id, previous } => {
                    let Some(task) = self.tasks.get_mut(&id) else { continue };
                    task.completed_at = previous;
                    self.save_task(id);
                    return true;
                }
                UndoEntry::Moved { id, project_id, tags } => {
                    if project_id.is_some_and(|p| self.project(p).is_none()) {
                        continue;
                    }
                    let Some(task) = self.tasks.get_mut(&id) else { continue };
                    task.project_id = project_id;
                    task.tags = tags;
                    self.save_task(id);
                    return true;
                }
            }
        }
        false
    }

    // ---------- internos ----------

    pub(crate) fn task_mut(&mut self, id: TaskId) -> Result<&mut Task, DomainError> {
        self.tasks.get_mut(&id).ok_or(DomainError::TaskNotFound(id))
    }

    /// Atualiza `updated_at` e enfileira a gravação da tarefa.
    pub(crate) fn save_task(&mut self, id: TaskId) {
        let now = self.clock.now_utc();
        if let Some(task) = self.tasks.get_mut(&id) {
            task.updated_at = now;
            self.ops.push(WriteOp::UpsertTask(task.clone()));
        }
    }

    pub(crate) fn remember(&mut self, entry: UndoEntry) {
        self.undo.push(entry);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    pub today: usize,
    pub upcoming: usize,
    pub inbox: usize,
    /// Há alguma tarefa aberta atrasada (data passada ou hora de hoje já passou).
    pub overdue: bool,
    pub per_project: HashMap<ProjectId, usize>,
}

impl Store {
    /// Tarefas da visão, já ordenadas. Concluídas só aparecem se estiverem em `keep`
    /// (a UI usa isso para manter a linha visível durante a animação de saída).
    pub fn view(&self, view: View, keep: &HashSet<TaskId>) -> Vec<&Task> {
        let today = self.today();
        let mut out: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| !t.is_done() || keep.contains(&t.id))
            .filter(|t| match view {
                View::Today => t.due.is_some_and(|d| d.date <= today),
                View::Upcoming => t.due.is_some_and(|d| d.date > today),
                View::Inbox => t.project_id.is_none(),
                View::Project(id) => t.project_id == Some(id),
            })
            .collect();
        match view {
            View::Today | View::Upcoming => out.sort_by(|a, b| {
                let (da, db) = (a.due.expect("filtrado por data"), b.due.expect("filtrado por data"));
                (da.date >= today)
                    .cmp(&(db.date >= today)) // atrasadas primeiro
                    .then(da.date.cmp(&db.date))
                    .then(da.time.is_none().cmp(&db.time.is_none())) // com hora antes de sem hora
                    .then(da.time.cmp(&db.time))
                    .then(a.sort_order.cmp(&b.sort_order))
                    .then(a.id.cmp(&b.id))
            }),
            View::Inbox | View::Project(_) => out.sort_by_key(|t| (t.sort_order, t.id)),
        }
        out
    }

    pub fn completed_in_project(&self, project_id: ProjectId) -> Vec<&Task> {
        let mut done: Vec<&Task> = self.tasks.values().filter(|t| t.project_id == Some(project_id) && t.is_done()).collect();
        done.sort_by(|a, b| b.completed_at.cmp(&a.completed_at).then(b.id.cmp(&a.id)));
        done
    }

    pub fn is_late(&self, task: &Task) -> bool {
        let now = self.now();
        !task.is_done()
            && task.due.is_some_and(|d| d.date < now.date() || (d.date == now.date() && d.time.is_some_and(|t| t <= now.time())))
    }

    pub fn counts(&self) -> Counts {
        let today = self.today();
        let mut counts = Counts::default();
        for p in &self.projects {
            counts.per_project.insert(p.id, 0);
        }
        for task in self.tasks.values().filter(|t| !t.is_done()) {
            match task.due {
                Some(d) if d.date <= today => counts.today += 1,
                Some(_) => counts.upcoming += 1,
                None => {}
            }
            match task.project_id {
                None => counts.inbox += 1,
                Some(p) => *counts.per_project.entry(p).or_insert(0) += 1,
            }
            if self.is_late(task) {
                counts.overdue = true;
            }
        }
        counts
    }
}

/// Filtro de texto (`Ctrl+F`): título ou notas, sem diferenciar maiúsculas. Vazio casa com tudo.
pub fn matches_query(task: &Task, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || task.title.to_lowercase().contains(&q) || task.notes.to_lowercase().contains(&q)
}
