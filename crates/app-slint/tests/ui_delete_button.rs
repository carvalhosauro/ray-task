use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};

/// A lixeira da linha apaga a tarefa (mesma animação e toast do Delete) e o desfazer a traz de volta.
#[test]
fn trash_button_deletes_and_undo_restores() {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();
    ui.show().unwrap();

    actions.invoke_new_task();
    actions.invoke_commit_new("Comprar pão".into());
    actions.invoke_escape();
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(50));
    assert_eq!(ui.get_tasks().row_count(), 1);

    let trash: Vec<_> = ElementHandle::find_by_accessible_label(&ui, "Apagar tarefa").collect();
    assert_eq!(trash.len(), 1);
    trash[0].invoke_accessible_default_action();
    assert!(ui.get_tasks().row_data(0).unwrap().leaving);

    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
    assert_eq!(ui.get_tasks().row_count(), 0);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_text(), "Tarefa apagada");

    actions.invoke_toast_action();
    assert_eq!(ui.get_tasks().row_count(), 1);
}
