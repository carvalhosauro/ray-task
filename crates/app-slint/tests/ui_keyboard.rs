use ray_core::writer::WriterEvent;
use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind::{self, TOAST_RETRY};
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow, Picker};
use slint::{ComponentHandle, Model};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn keyboard_filter_clock_and_save_errors() {
    i_slint_backend_testing::init_no_event_loop();
    let clock = FixedClock::at("2026-10-05 13:59");
    let mut store = Store::new(Snapshot::default(), Box::new(clock.clone()));
    let a = store.create_task(View::Today, "Pagar boleto").unwrap() as i32;
    let b = store.create_task(View::Today, "Ligar pro João").unwrap() as i32;
    store.take_ops();
    let retried = Rc::new(Cell::new(0));
    let counter = retried.clone();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(move || counter.set(counter.get() + 1)));
    let actions = ui.global::<Actions>();

    // setas + Enter + Esc
    actions.invoke_move_selection(1);
    assert_eq!(ui.get_selected_id(), a);
    actions.invoke_move_selection(1);
    assert_eq!(ui.get_selected_id(), b);
    actions.invoke_expand_selected();
    assert_eq!(ui.get_expanded_id(), b);
    actions.invoke_escape();
    assert_eq!(ui.get_expanded_id(), -1);
    assert_eq!(ui.get_selected_id(), b);

    // Ctrl+D abre o popover da tarefa selecionada (depois de um frame)
    actions.invoke_date_selected();
    assert_eq!(ui.get_expanded_id(), b);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(20));
    assert_eq!(ui.global::<Picker>().get_open_for(), b);
    actions.invoke_escape(); // fecha popover
    assert_eq!(ui.global::<Picker>().get_open_for(), -1);
    actions.invoke_escape(); // fecha a tarefa

    // Ctrl+Enter conclui a selecionada; Ctrl+Z desfaz
    actions.invoke_toggle_selected();
    assert!(ui.get_tasks().row_data(1).unwrap().done);
    actions.invoke_undo();
    assert!(!ui.get_tasks().row_data(1).unwrap().done);

    // Ctrl+F filtra; Esc fecha e limpa
    actions.invoke_toggle_filter();
    assert!(ui.get_filter_visible());
    actions.invoke_filter_changed("joão".into());
    assert_eq!(ui.get_tasks().row_count(), 1);
    actions.invoke_escape();
    assert!(!ui.get_filter_visible());
    assert_eq!(ui.get_tasks().row_count(), 2);

    // Esc com diálogo aberto fecha só o diálogo
    actions.invoke_new_project();
    actions.invoke_escape();
    assert!(!ui.get_prompt_visible());
    assert_eq!(ui.get_selected_id(), b);

    // relógio: tarefa de 14:00 pulsa quando a hora chega
    binding.set_time_for_test(a as i64, "14:00");
    clock.set("2026-10-05 14:00");
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(61_000));
    let row = ui.get_tasks().row_data(0).unwrap();
    assert_eq!(row.id, a);
    assert!(row.pulse);
    assert_eq!(row.meta, "14:00 · agora");
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(800));
    assert!(!ui.get_tasks().row_data(0).unwrap().pulse);

    // Delete apaga a selecionada (com Desfazer)
    actions.invoke_delete_selected();
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(250));
    assert_eq!(ui.get_tasks().row_count(), 1);
    assert_eq!(ui.get_toast_text(), "Tarefa apagada");

    // falha de gravação: toast fixo com "Tentar de novo"
    bind::show_writer_event(&ui, &WriterEvent::Failed { error: "disco cheio".into(), pending: 1 });
    assert_eq!(ui.get_toast_kind(), TOAST_RETRY);
    assert_eq!(ui.get_toast_action(), "Tentar de novo");
    actions.invoke_toast_action();
    assert_eq!(retried.get(), 1);
    bind::show_writer_event(&ui, &WriterEvent::Failed { error: "disco cheio".into(), pending: 1 });
    bind::show_writer_event(&ui, &WriterEvent::Recovered);
    assert!(!ui.get_toast_visible());
}
