use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use ray_core::{FixedClock, Snapshot, Store};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::platform::WindowEvent;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn ms(n: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(n));
}

/// (topo, base) do item com esse rótulo, em coordenadas da janela. A busca ignora elementos
/// totalmente recortados (fora da área visível do Flickable): `None` = escondido.
fn span(ui: &AppWindow, label: &str) -> Option<(f32, f32)> {
    ElementHandle::find_by_accessible_label(ui, label).next().map(|e| {
        let (y, h) = (e.absolute_position().y, e.size().height);
        (y, y + h)
    })
}

/// Muitos projetos: a lista rola dentro da sua área e não passa por baixo de "Novo projeto".
/// Ctrl+4..9 num projeto fora da área rola a sidebar até ele.
#[test]
fn many_projects_scroll_inside_the_sidebar() {
    i_slint_backend_testing::init_no_event_loop();
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    for i in 0..20 {
        store.create_project(&format!("Projeto {i}")).unwrap();
    }
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    ui.window().set_size(LogicalSize::new(960.0, 420.0));
    ui.show().unwrap();
    ms(50);

    let (footer_top, _) = span(&ui, "Novo projeto").unwrap();
    let list_bottom = footer_top - 4.0; // a área da lista termina 4 px acima do rodapé
    assert_eq!(span(&ui, "Projeto 19"), None, "o fim da lista fica recortado na área da lista, não sob o rodapé");

    // Ctrl+9 (índice 8 = "Projeto 5", cortado pela borda): a sidebar rola até ele ficar inteiro
    let (_, before) = span(&ui, "Projeto 5").unwrap();
    assert!(before > list_bottom, "Projeto 5 começa cortado numa janela de 420 px: {before} > {list_bottom}");
    ui.global::<Actions>().invoke_select_nav(8);
    ms(400);
    let (top, bottom) = span(&ui, "Projeto 5").unwrap();
    assert!(top >= 52.0 && bottom <= list_bottom, "Projeto 5 visível após Ctrl+9: {top}..{bottom}");

    // roda do mouse sobre a sidebar rola até o fim; o rodapé não se move
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(100.0, 200.0),
        delta_x: 0.0,
        delta_y: -2000.0,
    });
    ms(500);
    let (_, last_bottom) = span(&ui, "Projeto 19").expect("último projeto alcançável rolando");
    assert!(last_bottom <= list_bottom, "último projeto acima do rodapé: {last_bottom} <= {list_bottom}");
    assert_eq!(span(&ui, "Novo projeto").unwrap().0, footer_top);
}
