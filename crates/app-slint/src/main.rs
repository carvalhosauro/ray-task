// No console window behind the app on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use ray_core::writer::{SqliteSink, Writer};
use ray_core::{db, Store, SystemClock};
use ray_task::controller::Controller;
use ray_task::paths::{self, LockError, Paths};
use ray_task::shortcut::Shortcut;
use ray_task::{backend, bind, logging, AppWindow};
use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let started = Instant::now();
    let paths = Paths::from_env();
    let _log = logging::init(&paths.log_dir);
    // RAM budget, spec §1: renderer de software por padrão; SLINT_BACKEND do usuário vence.
    // Antes de qualquer chamada ao Slint; depois do log, para registrar uma falha.
    if let Some(name) = backend::default_backend(std::env::var_os("SLINT_BACKEND").as_deref()) {
        if let Err(e) = slint::BackendSelector::new().backend_name(name.into()).select() {
            tracing::error!(%e, backend = name, "selecionar renderer");
            return Err(e);
        }
    }
    let ui = AppWindow::new()?;

    let _lock = match paths::acquire_lock(&paths.lock) {
        Ok(lock) => lock,
        Err(LockError::AlreadyRunning) => return fatal(&ui, "O ray-task já está aberto em outra janela.".into()),
        Err(LockError::Io(e)) => return fatal(&ui, format!("Não foi possível criar o lock em {}:\n{e}", paths.lock.display())),
    };
    if let Some(shortcut) = Shortcut::from_env() {
        match shortcut.ensure() {
            Ok(outcome) => tracing::debug!(?outcome, "atalho no menu"),
            Err(e) => tracing::warn!(%e, "criar atalho no menu"),
        }
    }

    let opened = db::open(&paths.db).and_then(|conn| db::load(&conn).map(|snapshot| (conn, snapshot)));
    let (conn, snapshot) = match opened {
        Ok(pair) => pair,
        Err(e) => {
            tracing::error!(%e, path = %paths.db.display(), "abrir banco");
            return fatal(&ui, format!("Não foi possível abrir o banco de dados:\n{}\n\n{e}", paths.db.display()));
        }
    };

    let weak = ui.as_weak();
    let writer = Writer::spawn(SqliteSink(conn), move |event| {
        let _ = weak.upgrade_in_event_loop(move |ui| bind::show_writer_event(&ui, &event));
    });
    let handle = writer.handle();
    let retry = writer.handle();
    let controller = Controller::new(Store::new(snapshot, Box::new(SystemClock)), Box::new(move |ops| handle.send(ops)));
    let binding = bind::bind(&ui, controller, Box::new(move || retry.retry()));
    ui.invoke_focus_root();
    tracing::info!(ms = started.elapsed().as_millis() as u64, "pronto");

    ui.run()?;
    drop(binding);
    match writer.shutdown(Duration::from_secs(2)) {
        Some(0) => {}
        Some(pending) => tracing::error!(pending, "gravações não concluídas ao fechar"),
        None => tracing::error!("a fila de gravação não terminou em 2 s"),
    }
    Ok(())
}

fn fatal(ui: &AppWindow, message: String) -> Result<(), slint::PlatformError> {
    ui.set_fatal_error(message.into());
    ui.run()
}
