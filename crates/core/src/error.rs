use crate::model::{ProjectId, TagId, TaskId};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("O título não pode ficar vazio.")]
    EmptyTitle,
    #[error("O nome não pode ficar vazio.")]
    EmptyName,
    #[error("Projeto {0} não existe.")]
    ProjectNotFound(ProjectId),
    #[error("Tarefa {0} não existe.")]
    TaskNotFound(TaskId),
    #[error("Tag {0} não existe.")]
    TagNotFound(TagId),
    #[error("Já existe uma tag com esse nome neste projeto.")]
    DuplicateTag,
    #[error("A tag pertence a outro projeto.")]
    TagProjectMismatch,
}
