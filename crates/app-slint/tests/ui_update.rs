use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow, Prefs};
use semver::Version;
use slint::ComponentHandle;

const FIXTURE: &str = include_str!("fixtures/github-release.json");

fn setup() -> (AppWindow, bind::Binding) {
    i_slint_backend_testing::init_no_event_loop();
    let store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-06 10:00")));
    let mut ctrl = Controller::new(store, Box::new(|_| {}));
    ctrl.current_version = Version::new(0, 1, 0);
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, ctrl, Box::new(|| {}));
    (ui, binding)
}

#[test]
fn manual_check_shows_status_and_notice() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    let prefs = ui.global::<Prefs>();

    actions.invoke_check_updates();
    assert_eq!(prefs.get_update_status(), "Verificando…");
    actions.invoke_update_response(true, FIXTURE.into());
    assert_eq!(prefs.get_update_status(), "Versão 0.2.0 disponível");
    assert_eq!(prefs.get_notice_version(), "0.2.0");
    assert_eq!(prefs.get_notice_notes(), "• add a trash button to task rows\n• settings page with light and dark theme");
}

#[test]
fn manual_check_failure_is_shown() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    actions.invoke_check_updates();
    actions.invoke_update_response(false, "timeout".into());
    assert_eq!(ui.global::<Prefs>().get_update_status(), "Não foi possível verificar");
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
}

#[test]
fn foreign_url_is_treated_as_a_failure() {
    let (ui, _binding) = setup();
    let actions = ui.global::<Actions>();
    actions.invoke_check_updates();
    let json = FIXTURE.replace("https://github.com/carvalhosauro/ray-task/releases/tag/v0.2.0", "https://evil.example/x");
    actions.invoke_update_response(true, json.into());
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
    assert_eq!(ui.global::<Prefs>().get_update_status(), "Não foi possível verificar");
}

#[test]
fn notice_appears_in_the_sidebar_and_can_be_dismissed() {
    let (ui, binding) = setup();
    ui.show().unwrap();
    let actions = ui.global::<Actions>();
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 0);

    actions.invoke_check_updates();
    actions.invoke_update_response(true, FIXTURE.into());
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 1);

    actions.invoke_dismiss_update();
    assert_eq!(ui.global::<Prefs>().get_notice_version(), "");
    assert_eq!(ElementHandle::find_by_accessible_label(&ui, "v0.2.0 disponível").count(), 0);
    assert_eq!(binding.controller().store.settings().update_dismissed.as_deref(), Some("0.2.0"));
}

#[test]
fn install_command_is_exposed_to_the_ui() {
    let (ui, _binding) = setup();
    assert_eq!(ui.global::<Prefs>().get_install_command(), ray_task::update::install_command());
}
