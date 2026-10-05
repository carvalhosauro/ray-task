use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};

#[test]
fn project_management_flow() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    let casa = store.create_project("Casa").unwrap();
    let task = store.create_task(View::Project(casa), "Limpar garagem").unwrap();
    let dev = store.ensure_tag(casa, "dev").unwrap();
    store.add_tag(task, dev).unwrap();
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();

    // novo projeto
    actions.invoke_new_project();
    assert!(ui.get_prompt_visible());
    assert_eq!(ui.get_prompt_title(), "Novo projeto");
    actions.invoke_prompt_accept("   ".into());
    assert!(ui.get_prompt_visible(), "nome vazio mantém o diálogo aberto");
    assert_eq!(ui.get_prompt_error(), "O nome não pode ficar vazio.");
    actions.invoke_prompt_accept("Trabalho".into());
    assert!(!ui.get_prompt_visible());
    assert_eq!(ui.get_nav().row_count(), 5);
    assert_eq!(ui.get_view_title(), "Trabalho");

    // renomear e cor
    actions.invoke_begin_rename_project(casa as i32);
    assert_eq!(ui.get_prompt_text(), "Casa");
    actions.invoke_prompt_accept("Lar".into());
    assert_eq!(ui.get_nav().row_data(3).unwrap().label, "Lar");
    actions.invoke_set_project_color(casa as i32, 3);
    assert_eq!(binding.controller().store.project(casa).unwrap().color, "#30D158");

    // renomear tag para nome inválido e cancelar
    actions.invoke_begin_rename_tag(dev as i32);
    assert_eq!(ui.get_prompt_text(), "dev");
    actions.invoke_dialog_cancel();
    assert!(!ui.get_prompt_visible());

    // apagar tag com confirmação
    actions.invoke_request_delete_tag(dev as i32);
    assert!(ui.get_confirm_visible());
    assert_eq!(ui.get_confirm_title(), "Apagar a tag “dev”?");
    actions.invoke_confirm_accept();
    assert!(binding.controller().store.tag(dev).is_none());

    // apagar projeto com confirmação mostrando quantas tarefas
    actions.invoke_request_delete_project(casa as i32);
    assert_eq!(ui.get_confirm_title(), "Apagar “Lar”?");
    assert_eq!(ui.get_confirm_message(), "1 tarefa será apagada junto. Isso não pode ser desfeito.");
    assert_eq!(ui.get_confirm_button(), "Apagar");
    actions.invoke_dialog_cancel();
    assert!(binding.controller().store.project(casa).is_some());
    actions.invoke_request_delete_project(casa as i32);
    actions.invoke_confirm_accept();
    assert!(!ui.get_confirm_visible());
    assert_eq!(ui.get_nav().row_count(), 4);
    assert!(binding.controller().store.task(task).is_none());
}
