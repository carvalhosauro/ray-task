use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};

/// A lista é virtual: só as linhas visíveis existem. O teclado pede à lista que role até a
/// linha alvo (reveal-index + reveal-request); a rolagem em si acontece no Slint, com layout real.
#[test]
fn keyboard_reveals_target_row() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    for i in 0..50 {
        store.create_task(View::Today, &format!("Tarefa {i}")).unwrap();
    }
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();
    assert_eq!(ui.get_reveal_index(), -1);

    // ↓ até uma linha bem abaixo da primeira tela
    for _ in 0..40 {
        actions.invoke_move_selection(1);
    }
    let tasks = ui.get_tasks();
    assert_eq!(tasks.row_data(39).unwrap().id, ui.get_selected_id());
    assert_eq!(ui.get_reveal_index(), 39);
    assert_eq!(ui.get_reveal_request(), 40, "cada tecla pede de novo");

    // ↑ volta
    actions.invoke_move_selection(-30);
    assert_eq!(ui.get_reveal_index(), 9);

    // Ctrl+D e Ctrl+T também revelam a linha alvo antes de abrir
    let before = ui.get_reveal_request();
    actions.invoke_date_selected();
    assert_eq!(ui.get_reveal_index(), 9);
    assert_eq!(ui.get_reveal_request(), before + 1);
    actions.invoke_escape();
    actions.invoke_move_selection(20);
    actions.invoke_tag_selected();
    assert_eq!(ui.get_reveal_index(), 29);
}
