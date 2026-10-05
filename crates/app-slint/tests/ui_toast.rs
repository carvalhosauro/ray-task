use std::time::Duration;

use ray_core::writer::WriterEvent;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind::{self, TOAST_RETRY, TOAST_UNDO};
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};

/// O toast de falha de gravação fica visível até a falha ser resolvida.
#[test]
fn retry_toast_stays_until_resolved() {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();

    actions.invoke_new_task();
    actions.invoke_commit_new("Comprar pão".into());
    actions.invoke_commit_new("Pagar boleto".into());
    let first = ui.get_tasks().row_data(0).unwrap().id;
    let second = ui.get_tasks().row_data(1).unwrap().id;

    // toast de desfazer com timer de 5 s pendente
    actions.invoke_toggle(first);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_kind(), TOAST_UNDO);

    // falha de gravação: o timer antigo não pode esconder o toast
    bind::show_writer_event(&ui, &WriterEvent::Failed { error: "disco cheio".into(), pending: 1 });
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(5));
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_kind(), TOAST_RETRY);
    assert_eq!(ui.get_toast_text(), "Falha ao salvar");

    // concluir outra tarefa não troca o toast de falha
    actions.invoke_toggle(second);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_kind(), TOAST_RETRY);
    assert_eq!(ui.get_toast_text(), "Falha ao salvar");

    // Ctrl+Z desfaz, mas não esconde o toast de falha
    actions.invoke_undo();
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_kind(), TOAST_RETRY);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(6));
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_kind(), TOAST_RETRY);

    // recuperou: some
    bind::show_writer_event(&ui, &WriterEvent::Recovered);
    assert!(!ui.get_toast_visible());
}
