use std::time::{Duration, Instant};

use ray_core::{FixedClock, Snapshot, Store, View};
use ray_task::bind;
use ray_task::controller::Controller;
use ray_task::{Actions, AppWindow};
use slint::{ComponentHandle, Model};

fn titles(ui: &AppWindow) -> Vec<String> {
    ui.get_tasks().iter().map(|t| t.title.to_string()).collect()
}

/// Hoje e Próximos nunca têm tarefas em comum. Trocar entre eles substitui a lista inteira:
/// as linhas têm de sair certas e a troca não pode custar n_novas × n_antigas.
#[test]
fn switching_between_disjoint_views() {
    i_slint_backend_testing::init_no_event_loop();
    const N: usize = 2000;
    let mut store = Store::new(Snapshot::default(), Box::new(FixedClock::at("2026-10-05 13:35")));
    for i in 0..N {
        store.create_task(View::Today, &format!("Hoje {i}")).unwrap();
        store.create_task(View::Upcoming, &format!("Amanhã {i}")).unwrap();
    }
    store.take_ops();
    let ui = AppWindow::new().unwrap();
    let _binding = bind::bind(&ui, Controller::new(store, Box::new(|_| {})), Box::new(|| {}));
    let actions = ui.global::<Actions>();
    let today: Vec<String> = (0..N).map(|i| format!("Hoje {i}")).collect();
    let upcoming: Vec<String> = (0..N).map(|i| format!("Amanhã {i}")).collect();
    assert_eq!(titles(&ui), today);

    let started = Instant::now();
    actions.invoke_select_nav(1);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(100));
    let to_upcoming = started.elapsed();
    assert_eq!(ui.get_view_title(), "Próximos");
    assert_eq!(titles(&ui), upcoming);
    assert_eq!(ui.get_tasks().row_data(0).unwrap().group_header, "Amanhã", "cabeçalho do dia na primeira linha");

    let started = Instant::now();
    actions.invoke_select_nav(0);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(100));
    let to_today = started.elapsed();
    assert_eq!(titles(&ui), today);
    println!("troca Hoje→Próximos: {to_upcoming:?}; Próximos→Hoje: {to_today:?} ({N} linhas cada)");
    // Folga larga para build debug numa máquina fraca; o algoritmo quadrático passa de segundos.
    assert!(to_upcoming + to_today < Duration::from_millis(1500), "troca de visão lenta: {to_upcoming:?} + {to_today:?}");

    // Mudança no lugar (mesma visão) continua certa: apagar uma linha do meio e desfazer.
    let middle = ui.get_tasks().row_data(N / 2).unwrap().id;
    actions.invoke_delete(middle);
    assert!(ui.get_tasks().row_data(N / 2).unwrap().leaving, "a linha fica para animar a saída");
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
    assert_eq!(ui.get_tasks().row_count(), N - 1);
    actions.invoke_undo();
    assert_eq!(titles(&ui), today);
}
