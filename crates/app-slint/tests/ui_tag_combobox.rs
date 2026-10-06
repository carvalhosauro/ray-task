use std::time::Duration;

use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

fn tap(ui: &AppWindow, key: impl Into<SharedString>) {
    let key: SharedString = key.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: key });
    ms(20);
}

fn type_text(ui: &AppWindow, text: &str) {
    for c in text.chars() {
        tap(ui, c.to_string());
    }
}

fn tags(ui: &AppWindow, row: usize) -> Vec<String> {
    ui.get_tasks().row_data(row).unwrap().tags.iter().map(|t| t.name.to_string()).collect()
}

fn options(actions: &Actions<'_>) -> Vec<String> {
    actions.get_tag_options().iter().map(|s| s.to_string()).collect()
}

/// Projeto com as `names` como tags (numa tarefa à parte); a tarefa alvo fica na primeira linha.
fn setup(names: &[&str]) -> (AppWindow, bind::Binding, i64) {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let p = store.create_project("ray-task").unwrap();
    let id = store.create_task(View::Project(p), "Revisar PR").unwrap();
    let holder = store.create_task(View::Project(p), "Guarda as tags").unwrap();
    for name in names {
        let tag = store.ensure_tag(p, name).unwrap();
        store.add_tag(holder, tag).unwrap();
    }
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.global::<Actions>().invoke_select_nav(3);
    ms(100);
    assert_eq!(ui.get_tasks().row_data(0).unwrap().id as i64, id);
    (ui, binding, id)
}

/// ↓ seleciona a primeira tarefa e Ctrl+T foca o campo de tag (dois saltos de timer de 16 ms).
fn focus_tag_field(ui: &AppWindow) {
    ui.invoke_focus_root();
    tap(ui, Key::DownArrow);
    ui.global::<Actions>().invoke_tag_selected();
    for _ in 0..3 {
        ms(20);
    }
}

#[test]
fn tag_field_lists_filters_and_picks() {
    let (ui, _binding, id) = setup(&["casa", "carro", "ui-kit"]);
    let actions = ui.global::<Actions>();
    focus_tag_field(&ui);
    assert!(actions.get_tag_list_open(), "Ctrl+T abre a lista");
    assert_eq!(options(&actions), ["carro", "casa", "ui-kit"]);
    assert_eq!(actions.get_tag_highlighted(), -1);

    type_text(&ui, "ca");
    assert_eq!(options(&actions), ["carro", "casa"]);
    tap(&ui, Key::DownArrow);
    assert_eq!(actions.get_tag_highlighted(), 0);
    tap(&ui, Key::Return);
    assert_eq!(tags(&ui, 0), ["carro"]);
    assert!(!actions.get_tag_list_open(), "Enter fecha a lista");

    type_text(&ui, "kit");
    assert_eq!(options(&actions), ["ui-kit"], "contém também entra");
    tap(&ui, Key::Escape);
    assert!(!actions.get_tag_list_open(), "o primeiro Esc fecha só a lista");
    assert_eq!(ui.get_expanded_id() as i64, id);
    tap(&ui, Key::Escape);
    assert_eq!(ui.get_expanded_id(), -1, "o segundo Esc fecha a tarefa");

    focus_tag_field(&ui);
    actions.invoke_tag_pick(id as i32, "casa".into());
    ms(20);
    assert_eq!(tags(&ui, 0), ["carro", "casa"]);
    assert!(!actions.get_tag_list_open());
}

#[test]
fn escape_in_field_without_tags_reaches_the_app() {
    let (ui, _binding, id) = setup(&[]);
    focus_tag_field(&ui);
    assert_eq!(ui.get_expanded_id() as i64, id);
    tap(&ui, Key::Escape);
    assert_eq!(ui.get_expanded_id(), -1, "sem lista à vista, o Esc fecha a tarefa");
}
