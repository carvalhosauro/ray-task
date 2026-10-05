use ray_core::db;
use ray_task::paths::{self, LockError, Paths};
use ray_task::{logging, AppWindow};
use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let paths = Paths::from_env();
    let _log = logging::init(&paths.log_dir);
    let ui = AppWindow::new()?;
    let _lock = match paths::acquire_lock(&paths.lock) {
        Ok(lock) => lock,
        Err(LockError::AlreadyRunning) => {
            ui.set_fatal_error("O ray-task já está aberto em outra janela.".into());
            return ui.run();
        }
        Err(LockError::Io(e)) => {
            ui.set_fatal_error(format!("Não foi possível criar o lock em {}:\n{e}", paths.lock.display()).into());
            return ui.run();
        }
    };
    match db::open(&paths.db).and_then(|conn| db::load(&conn)) {
        Ok(snapshot) => ui.set_status(format!("{} tarefas carregadas", snapshot.tasks.len()).into()),
        Err(e) => {
            tracing::error!(%e, "abrir banco");
            ui.set_fatal_error(format!("Não foi possível abrir o banco de dados:\n{}\n\n{e}", paths.db.display()).into());
        }
    }
    ui.run()
}
