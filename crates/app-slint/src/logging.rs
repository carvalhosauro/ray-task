use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;

/// Log em arquivo, nível info. Devolve o guard que precisa viver até o fim do `main`.
pub fn init(dir: &Path) -> Option<WorkerGuard> {
    std::fs::create_dir_all(dir).ok()?;
    let appender = tracing_appender::rolling::never(dir, "ray-task.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt().with_writer(writer).with_ansi(false).with_max_level(tracing::Level::INFO).init();
    Some(guard)
}
