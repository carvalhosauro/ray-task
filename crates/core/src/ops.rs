use crate::model::{Project, ProjectId, Tag, TagId, Task, TaskId};

/// Uma gravação pendente. `UpsertTask` grava a linha e substitui as tags da tarefa.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteOp {
    UpsertProject(Project),
    DeleteProject(ProjectId),
    UpsertTag(Tag),
    DeleteTag(TagId),
    UpsertTask(Task),
    DeleteTask(TaskId),
}
