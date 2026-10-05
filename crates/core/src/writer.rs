use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusqlite::Connection;

use crate::db;
use crate::ops::WriteOp;

/// Destino das gravações. Em produção é o SQLite; nos testes, um disco falso.
pub trait Sink: Send + 'static {
    fn apply(&mut self, op: &WriteOp) -> Result<(), String>;
}

pub struct SqliteSink(pub Connection);

impl Sink for SqliteSink {
    fn apply(&mut self, op: &WriteOp) -> Result<(), String> {
        db::apply(&mut self.0, op).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriterEvent {
    /// Uma gravação falhou; ela e as seguintes continuam na fila.
    Failed { error: String, pending: usize },
    /// A fila voltou a esvaziar depois de uma falha.
    Recovered,
}

enum Msg {
    Ops(Vec<WriteOp>),
    Retry,
    Shutdown(Sender<usize>),
}

#[derive(Clone)]
pub struct WriterHandle {
    tx: Sender<Msg>,
}

impl WriterHandle {
    pub fn send(&self, ops: Vec<WriteOp>) {
        if ops.is_empty() {
            return;
        }
        let lost = ops.len();
        if self.tx.send(Msg::Ops(ops)).is_err() {
            tracing::error!(lost, "thread de escrita encerrada; operações descartadas");
        }
    }

    pub fn retry(&self) {
        if self.tx.send(Msg::Retry).is_err() {
            tracing::error!("thread de escrita encerrada; retry ignorado");
        }
    }
}

pub struct Writer {
    handle: WriterHandle,
    thread: JoinHandle<()>,
}

impl Writer {
    pub fn spawn<S: Sink>(sink: S, on_event: impl Fn(WriterEvent) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("ray-writer".into())
            .spawn(move || run(sink, rx, on_event))
            .expect("não foi possível criar a thread de escrita");
        Self { handle: WriterHandle { tx }, thread }
    }

    pub fn handle(&self) -> WriterHandle {
        self.handle.clone()
    }

    /// Tenta gravar o que falta e encerra. `None` se não terminou dentro de `timeout`.
    pub fn shutdown(self, timeout: Duration) -> Option<usize> {
        let (ack_tx, ack_rx) = mpsc::channel();
        if self.handle.tx.send(Msg::Shutdown(ack_tx)).is_err() {
            return Some(0);
        }
        let pending = ack_rx.recv_timeout(timeout).ok()?;
        let _ = self.thread.join();
        Some(pending)
    }
}

fn run<S: Sink>(mut sink: S, rx: Receiver<Msg>, on_event: impl Fn(WriterEvent)) {
    let mut queue = VecDeque::new();
    let mut failing = false;
    for msg in rx {
        let ack = match msg {
            Msg::Ops(ops) => {
                queue.extend(ops);
                None
            }
            Msg::Retry => None,
            Msg::Shutdown(ack) => Some(ack),
        };
        flush(&mut sink, &mut queue, &mut failing, &on_event);
        if let Some(ack) = ack {
            let _ = ack.send(queue.len());
            return;
        }
    }
    if !queue.is_empty() {
        tracing::warn!(pending = queue.len(), "thread de escrita encerrou sem shutdown com operações pendentes");
    }
}

fn flush<S: Sink>(sink: &mut S, queue: &mut VecDeque<WriteOp>, failing: &mut bool, on_event: &impl Fn(WriterEvent)) {
    while let Some(op) = queue.front() {
        match sink.apply(op) {
            Ok(()) => {
                queue.pop_front();
            }
            Err(error) => {
                tracing::error!(%error, pending = queue.len(), "falha ao gravar");
                *failing = true;
                on_event(WriterEvent::Failed { error, pending: queue.len() });
                return;
            }
        }
    }
    if std::mem::replace(failing, false) {
        on_event(WriterEvent::Recovered);
    }
}
