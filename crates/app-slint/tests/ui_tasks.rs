use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};
use std::time::Duration;

#[test]
fn task_list_flow() {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();

    assert_eq!(ui.get_view_title(), "Hoje");
    assert_eq!(ui.get_nav().row_count(), 3);
    assert_eq!(ui.get_empty_text(), "Nada por aqui. Ctrl+N para capturar uma tarefa.");

    // criar
    actions.invoke_new_task();
    assert!(ui.get_adding());
    actions.invoke_commit_new("Comprar pão".into());
    actions.invoke_commit_new("Pagar boleto".into());
    let tasks = ui.get_tasks();
    assert_eq!(tasks.row_count(), 2);
    assert_eq!(tasks.row_data(0).unwrap().title, "Comprar pão");
    assert_eq!(ui.get_nav().row_data(0).unwrap().count, 2);
    let first = tasks.row_data(0).unwrap().id;

    // concluir: a linha fica (check preenchido), sai depois das animações
    actions.invoke_toggle(first);
    assert!(ui.get_tasks().row_data(0).unwrap().done);
    assert!(ui.get_toast_visible());
    assert_eq!(ui.get_toast_text(), "Tarefa concluída");
    assert_eq!(ui.get_nav().row_data(0).unwrap().count, 1);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(600));
    assert!(ui.get_tasks().row_data(0).unwrap().leaving);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
    assert_eq!(ui.get_tasks().row_count(), 1);

    // desfazer pelo toast
    actions.invoke_toast_action();
    assert_eq!(ui.get_tasks().row_count(), 2);
    assert!(!ui.get_toast_visible());

    // apagar (adiado até a animação) e desfazer
    actions.invoke_delete(first);
    assert!(ui.get_tasks().row_data(0).unwrap().leaving);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
    assert_eq!(ui.get_tasks().row_count(), 1);
    assert_eq!(ui.get_toast_text(), "Tarefa apagada");
    actions.invoke_undo();
    assert_eq!(ui.get_tasks().row_count(), 2);

    // expandir
    actions.invoke_expand(first);
    assert_eq!(ui.get_expanded_id(), first);
    actions.invoke_expand(first);
    assert_eq!(ui.get_expanded_id(), -1);

    // trocar de visão (crossfade de 120 ms no total)
    actions.invoke_select_nav(2);
    assert_eq!(ui.get_selected_nav(), 2);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(100));
    assert_eq!(ui.get_view_title(), "Entrada");
    assert!(!ui.get_content_faded());
}
