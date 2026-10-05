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

- [x] Abertura < 200 ms: `grep pronto ~/.local/state/ray-task/ray-task.log | tail -1` → 34 ms (banco real), 79 ms (backend padrão, 5.000 tarefas), 51 ms (`winit-femtovg`, 5.000 tarefas)
- [ ] RAM < 40 MB com 5.000 tarefas: **FALHOU**. Popule um banco temporário com `cargo run -p ray-core --release --example seed -- <caminho.db>` (use `XDG_DATA_HOME`/`XDG_STATE_HOME`/`XDG_RUNTIME_DIR` temporários, com symlink do socket `wayland-0` no runtime dir) e meça com `/usr/bin/time -v target/release/ray-task` → "Maximum resident set size":

  | Backend | Banco vazio | 5.000 tarefas |
  |---|---|---|
  | padrão | 85.924 KB | 137.272 KB |
  | `winit-femtovg` | n/d | 138.172 KB |
  | `winit-skia` | n/d | 137.792 KB |
  | `winit-software` | 35.280 KB | 93.232 KB |

  Leitura: o renderer GPU custa ~50 MB de base; as 5.000 tarefas custam ~55 MB em qualquer renderer (`for item in root.tasks` em `app.slint` instancia um `TaskRow` por tarefa, sem virtualização).
- [x] Visões < 2 ms: saída do teste de perf → Hoje 104 µs, Próximos 187 µs, Entrada 14 µs, Projeto 20 µs, contagens 61 µs; abrir + carregar 6,0 ms
