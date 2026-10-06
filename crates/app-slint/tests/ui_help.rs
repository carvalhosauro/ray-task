use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::AppWindow;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

fn tap(ui: &AppWindow, key: impl Into<SharedString>) {
    let key: SharedString = key.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: key });
    ms(20);
}

fn ctrl(ui: &AppWindow, key: impl Into<SharedString>) {
    let control: SharedString = Key::Control.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    tap(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn setup() -> (AppWindow, bind::Binding) {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();
    ms(50);
    (ui, binding)
}

#[test]
fn f1_and_question_mark_open_the_overlay_and_esc_closes_it() {
    let (ui, _binding) = setup();
    tap(&ui, Key::F1);
    assert!(ui.get_help_open());
    assert!(ElementHandle::find_by_accessible_label(&ui, "Atalhos").next().is_some());
    tap(&ui, Key::Escape);
    assert!(!ui.get_help_open());
    tap(&ui, "?");
    assert!(ui.get_help_open());
    tap(&ui, "?");
    assert!(!ui.get_help_open(), "? de novo fecha");
}

#[test]
fn question_mark_in_a_text_field_is_just_text() {
    let (ui, binding) = setup();
    ctrl(&ui, "f");
    ms(50);
    tap(&ui, "?");
    assert!(!ui.get_help_open());
    assert_eq!(binding.controller().filter, "?");
}

#[test]
fn settings_page_lists_the_shortcuts() {
    let (ui, _binding) = setup();
    ctrl(&ui, ",");
    ms(100);
    let rows: Vec<_> = ElementHandle::find_by_accessible_label(&ui, "Ctrl+N").collect();
    assert_eq!(rows.len(), 1, "linha Ctrl+N na seção Atalhos");
}
