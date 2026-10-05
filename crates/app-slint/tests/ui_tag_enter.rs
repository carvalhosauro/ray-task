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

/// Enter no campo de tag usa o texto digitado; Tab aceita a sugestão (spec §6: "Enter cria tag nova").
#[test]
fn tag_field_enter_uses_typed_text_and_tab_takes_suggestion() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let p = store.create_project("ray-task").unwrap();
    let id = store.create_task(View::Project(p), "Revisar PR").unwrap();
    let other = store.create_task(View::Project(p), "Outra").unwrap();
    let kit = store.ensure_tag(p, "ui-kit").unwrap();
    store.add_tag(other, kit).unwrap();
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();
    ui.show().unwrap();
    actions.invoke_select_nav(3);
    ms(100);
    assert_eq!(ui.get_tasks().row_data(0).unwrap().id as i64, id);
    assert_eq!(actions.invoke_tag_suggest(id as i32, "ui".into()), "ui-kit");

    // ↓ seleciona, Ctrl+T foca o campo de tag
    ui.invoke_focus_root();
    tap(&ui, Key::DownArrow);
    actions.invoke_tag_selected();
    // pedido de foco: timer do bind (16 ms) → timer dos detalhes recém-criados (16 ms)
    for _ in 0..3 {
        ms(20);
    }

    type_text(&ui, "ui");
    tap(&ui, Key::Return);
    assert_eq!(tags(&ui, 0), ["ui"], "Enter cria a tag digitada, não a sugestão ui-kit");

    // Tab completa com a sugestão; Enter confirma
    type_text(&ui, "u");
    tap(&ui, Key::Tab);
    tap(&ui, Key::Return);
    assert_eq!(tags(&ui, 0), ["ui", "ui-kit"]);

    let ids: Vec<i32> = ui.get_tasks().row_data(0).unwrap().tags.iter().map(|t| t.id).collect();
    assert_eq!(ids[1], kit as i32, "Tab reaproveita a tag existente");
}
