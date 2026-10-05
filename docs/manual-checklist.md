# Checklist manual (antes de cada release)

Compare lado a lado com `docs/superpowers/mockups/2026-10-05-ui-preview.html`.

## Visual
- [ ] Cores claro/escuro iguais à tabela de tokens da spec (§6)
- [ ] Inter em todos os textos; título da visão 26 px bold
- [ ] Linha aberta: cartão branco com sombra suave, raio 10 px
- [ ] Popover de data: raio 14 px, sombra, calendário começando no domingo

## Animações
- [ ] Concluir: check preenche (150 ms), linha some após ~600 ms (200 ms)
- [ ] Desfazer pelo toast dentro dos 600 ms: a linha não some
- [ ] Expandir/fechar: altura + opacidade, 200 ms
- [ ] Nova tarefa: linha entra crescendo
- [ ] Trocar visão: crossfade curto; destaque da sidebar desliza
- [ ] Apagar: linha colapsa e aparece "Tarefa apagada · Desfazer" por 5 s
- [ ] Hora chegando com o app aberto: pulso laranja uma vez

## Comportamento
- [ ] Todos os atalhos do README
- [ ] Mover tarefa com tags pede confirmação; sem tags, move direto
- [ ] Apagar projeto mostra quantas tarefas vão junto
- [ ] Segunda instância mostra "O ray-task já está aberto em outra janela."
- [ ] `chmod 000` no banco → tela de erro com caminho e motivo (desfaça depois)
- [ ] Deixar o app aberto na virada da meia-noite: Amanhã vira Hoje

## Orçamentos (anote os valores)

Medido em 2026-10-05, build release, Fedora 43 / Wayland (KWin).

- [x] Abertura < 200 ms: `grep pronto ~/.local/state/ray-task/ray-task.log | tail -1` → 34 ms (banco real); 5.000 tarefas: 37–38 ms (padrão, software), 39 ms (`winit-femtovg`)
- [x] RAM < 40 MB com 5.000 tarefas (backend padrão = `winit-software`, lista virtualizada): popule um banco temporário com `cargo run -p ray-core --release --example seed -- <caminho.db>` (use `XDG_DATA_HOME`/`XDG_STATE_HOME`/`XDG_RUNTIME_DIR` temporários e `WAYLAND_DISPLAY=/run/user/1000/wayland-0`) e meça com `/usr/bin/time -v target/release/ray-task` → "Maximum resident set size":

  | Backend | Banco vazio | 5.000 tarefas |
  |---|---|---|
  | padrão (`winit-software`) | 35.000–35.572 KB | 38.188–39.280 KB |
  | `winit-femtovg` (GPU, opt-in) | 85.668 KB | 88.944 KB |

  Leitura: a lista só instancia as linhas visíveis; 5.000 tarefas custam ~4 MB (dados no Rust). O renderer GPU custa ~50 MB de base, por isso o padrão é software. Margem: ~1 MB.
- [x] Visões < 2 ms: saída do teste de perf → Hoje 104 µs, Próximos 187 µs, Entrada 14 µs, Projeto 20 µs, contagens 61 µs; abrir + carregar 6,0 ms
