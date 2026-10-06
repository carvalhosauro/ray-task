use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Settings, Snapshot, Store, ThemeMode, View, WriteOp};
use ray_task::bind;
use ray_task::controller::{Controller, Page};
use ray_task::{Actions, AppWindow, Theme};
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

fn ctrl(ui: &AppWindow, key: impl Into<SharedString>) {
    let control: SharedString = Key::Control.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    tap(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn click(ui: &AppWindow, label: &str) {
    let found: Vec<_> = ElementHandle::find_by_accessible_label(ui, label).collect();
    assert_eq!(found.len(), 1, "um elemento com o rótulo {label:?}");
    found[0].invoke_accessible_default_action();
}

#[test]
fn settings_page_switches_theme_and_persists_it() {
    i_slint_backend_testing::init_no_event_loop();
    let sent = Rc::new(RefCell::new(Vec::<WriteOp>::new()));
    let sink = sent.clone();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(move |ops| sink.borrow_mut().extend(ops))), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();

    ctrl(&ui, ",");
    ms(100);
    assert_eq!(ui.get_page(), 1);
    assert_eq!(ui.get_selected_nav(), -1, "nenhuma visão marcada nas configurações");

    click(&ui, "Escuro");
    assert_eq!(ui.global::<Theme>().get_mode(), 2);
    assert!(ui.global::<Theme>().get_dark());
    assert_eq!(binding.controller().store.settings().theme, ThemeMode::Dark);
    assert!(sent.borrow().contains(&WriteOp::SetSetting { key: "theme", value: "dark".into() }));

    click(&ui, "Claro");
    assert!(!ui.global::<Theme>().get_dark());

    tap(&ui, Key::Escape);
    assert_eq!(ui.get_page(), 0);
    assert_eq!(binding.controller().page, Page::Tasks);
}

#[test]
fn saved_theme_is_applied_before_the_window_shows() {
    i_slint_backend_testing::init_no_event_loop();
    let snapshot = Snapshot { settings: Settings { theme: ThemeMode::Dark, ..Settings::default() }, ..Snapshot::default() };
    let store = Store::new(snapshot, Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    assert_eq!(ui.global::<Theme>().get_mode(), 2);
    assert!(ui.global::<Theme>().get_dark());
}

#[test]
fn delete_on_the_settings_page_does_not_touch_the_hidden_task() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    store.create_task(View::Today, "Comprar pão").unwrap();
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.invoke_focus_root();
    ms(50);

    tap(&ui, Key::DownArrow);
    assert!(binding.controller().selected.is_some());
    ui.global::<Actions>().invoke_open_settings();
    ms(100);
    tap(&ui, Key::Delete);
    ms(300);
    assert_eq!(ui.get_tasks().row_count(), 1, "a tarefa escondida continua lá");
}

#[test]
fn update_check_switch_persists() {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.show().unwrap();
    ui.global::<Actions>().invoke_open_settings();
    ms(100);
    click(&ui, "Verificar automaticamente");
    assert!(!binding.controller().store.settings().update_check);
}
