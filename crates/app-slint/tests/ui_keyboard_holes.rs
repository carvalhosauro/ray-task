use std::time::Duration;

use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{AppWindow, Picker};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

/// Pedidos de foco/popover passam por até dois timers de 16 ms.
fn settle() {
    for _ in 0..3 {
        ms(20);
    }
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

fn ctrl(ui: &AppWindow, key: impl Into<SharedString>) {
    let control: SharedString = Key::Control.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    tap(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

/// Fluxo diário só com teclado (spec §1): projeto por Ctrl+4..9, título focado ao abrir com
/// Enter, popover de data operável por teclas.
#[test]
fn keyboard_only_daily_flow() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    store.create_project("Casa").unwrap();
    let p = store.create_project("ray-task").unwrap();
    let id = store.create_task(View::Project(p), "Revisar PR").unwrap() as i32;
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let picker = ui.global::<Picker>();
    ui.show().unwrap();
    ui.invoke_focus_root();
    ms(50);

    // Ctrl+4 / Ctrl+5: projetos na ordem da sidebar; Ctrl+9 sem projeto não faz nada
    ctrl(&ui, "4");
    ms(100);
    assert_eq!(ui.get_view_title(), "Casa");
    ctrl(&ui, "5");
    ms(100);
    assert_eq!(ui.get_view_title(), "ray-task");
    ctrl(&ui, "9");
    ms(100);
    assert_eq!(ui.get_view_title(), "ray-task");

    // ↓ + Enter abre a tarefa com o foco no título
    tap(&ui, Key::DownArrow);
    tap(&ui, Key::Return);
    settle();
    assert_eq!(ui.get_expanded_id(), id);
    type_text(&ui, "!");
    let title = binding.controller().store.task(id as i64).unwrap().title.clone();
    assert!(title.contains('!') && title.contains("Revisar PR"), "digitar após Enter edita o título: {title:?}");
    tap(&ui, Key::Escape);
    assert_eq!(ui.get_expanded_id(), -1);
    assert_eq!(ui.get_selected_id(), id);

    // Ctrl+D: o campo de hora recebe o foco
    let row = |ui: &AppWindow| ui.get_tasks().row_data(0).unwrap();
    ctrl(&ui, "d");
    settle();
    assert_eq!(picker.get_open_for(), id);
    type_text(&ui, "9:30");
    tap(&ui, Key::Return);
    assert!(row(&ui).time_label.starts_with("09:30"), "hora digitada no campo focado: {}", row(&ui).time_label);
    assert_eq!(row(&ui).date_label, "Hoje", "hora sem data assume hoje");

    // teclas de atalho dentro do popover
    tap(&ui, "a");
    assert_eq!(row(&ui).date_label, "Amanhã");
    assert_eq!(picker.get_open_for(), -1, "escolher fecha o popover");

    ctrl(&ui, "d");
    settle();
    tap(&ui, "S");
    assert_eq!(row(&ui).date_label, "Seg, 12 out");

    ctrl(&ui, "d");
    settle();
    tap(&ui, "h");
    assert_eq!(row(&ui).date_label, "Hoje");

    ctrl(&ui, "d");
    settle();
    tap(&ui, Key::Delete);
    assert_eq!(row(&ui).date_label, "Sem data");

    // Esc fecha o popover (e só ele)
    ctrl(&ui, "d");
    settle();
    assert_eq!(picker.get_open_for(), id);
    tap(&ui, Key::Escape);
    assert_eq!(picker.get_open_for(), -1);
    assert_eq!(ui.get_expanded_id(), id);
    tap(&ui, Key::Escape);
    assert_eq!(ui.get_expanded_id(), -1);
}
