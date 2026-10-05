use std::time::Duration;

use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow, Picker};
use slint::{ComponentHandle, Model};

#[test]
fn details_flow() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let p = store.create_project("ray-task").unwrap();
    let id = store.create_task(View::Project(p), "Revisar PR").unwrap() as i32;
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();
    let picker = ui.global::<Picker>();
    actions.invoke_select_nav(3);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(100));
    assert_eq!(ui.get_view_title(), "ray-task");

    // popover de data
    let before = picker.get_request();
    picker.invoke_open(id);
    assert_eq!(picker.get_open_for(), id);
    assert_eq!(picker.get_request(), before + 1);
    assert_eq!(ui.get_expanded_id(), id);
    assert_eq!(picker.get_month_title(), "Outubro 2026");
    assert_eq!(picker.get_next_week_hint(), "Seg, 12 out");
    picker.invoke_quick(id, 1);
    let row = ui.get_tasks().row_data(0).unwrap();
    assert_eq!(row.date_label, "Amanhã");
    assert_eq!(picker.get_open_for(), -1);

    picker.invoke_open(id);
    picker.invoke_shift_month(1);
    assert_eq!(picker.get_month_title(), "Novembro 2026");
    picker.invoke_pick(id, "2026-11-03".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().date_label, "Ter, 3 nov");

    picker.invoke_set_time(id, "9:5".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().time_label, "09:05");
    picker.invoke_set_time(id, "abc".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().time_label, "09:05");
    picker.invoke_set_time(id, "".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().time_label, "");

    // tags
    actions.invoke_add_tag(id, "#Dev".into());
    actions.invoke_add_tag(id, "docs".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().tags.row_count(), 2);
    actions.invoke_remove_last_tag(id);
    let tags = ui.get_tasks().row_data(0).unwrap().tags;
    assert_eq!(tags.row_count(), 1);
    assert_eq!(tags.row_data(0).unwrap().name, "Dev");
    assert_eq!(actions.invoke_tag_suggest(id, "de".into()), "", "a tarefa já tem Dev");
    assert_eq!(actions.invoke_tag_suggest(id, "do".into()), "docs");

    // mover com tags pede confirmação
    actions.invoke_request_move(id, -1);
    assert!(ui.get_confirm_visible());
    assert_eq!(ui.get_confirm_button(), "Mover");
    actions.invoke_confirm_accept();
    assert_eq!(binding.controller().store.task(id as i64).unwrap().project_id, None);
    assert_eq!(ui.get_tasks().row_count(), 0, "saiu da visão do projeto");

    // mover sem tags é direto
    actions.invoke_request_move(id, p as i32);
    assert!(!ui.get_confirm_visible());
    assert_eq!(ui.get_tasks().row_count(), 1);

    // editar texto
    actions.invoke_edit(id, "Revisar PR #12".into(), "checar trigger".into());
    let row = ui.get_tasks().row_data(0).unwrap();
    assert_eq!((row.title.as_str(), row.notes.as_str()), ("Revisar PR #12", "checar trigger"));
    actions.invoke_edit(id, "   ".into(), "x".into());
    assert_eq!(ui.get_tasks().row_data(0).unwrap().title, "Revisar PR #12", "título vazio é ignorado");
}
