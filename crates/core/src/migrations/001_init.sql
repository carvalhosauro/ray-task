CREATE TABLE projects (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL,
  color       TEXT    NOT NULL,
  sort_order  INTEGER NOT NULL,
  created_at  TEXT    NOT NULL
);

CREATE TABLE tasks (
  id            INTEGER PRIMARY KEY,
  project_id    INTEGER NULL REFERENCES projects(id) ON DELETE CASCADE,
  title         TEXT    NOT NULL CHECK (length(trim(title)) > 0),
  notes         TEXT    NOT NULL DEFAULT '',
  due_date      TEXT    NULL,
  due_time      TEXT    NULL,
  completed_at  TEXT    NULL,
  sort_order    INTEGER NOT NULL,
  created_at    TEXT    NOT NULL,
  updated_at    TEXT    NOT NULL,
  CHECK (due_time IS NULL OR due_date IS NOT NULL)
);

CREATE TABLE tags (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name        TEXT    NOT NULL COLLATE NOCASE,
  color       TEXT    NOT NULL,
  UNIQUE (project_id, name)
);

CREATE TABLE task_tags (
  task_id  INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (task_id, tag_id)
);

CREATE INDEX tasks_project ON tasks(project_id);
CREATE INDEX tasks_due ON tasks(due_date);

-- Tag e tarefa precisam ser do mesmo projeto (tarefa na Entrada não tem tags).
CREATE TRIGGER task_tags_same_project BEFORE INSERT ON task_tags
BEGIN
  SELECT RAISE(ABORT, 'tag e tarefa de projetos diferentes')
  WHERE (SELECT project_id FROM tags WHERE id = NEW.tag_id)
        IS NOT (SELECT project_id FROM tasks WHERE id = NEW.task_id);
END;
