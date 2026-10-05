# ray-task

TODO desktop, local, leve e fluido. Rust + Slint + SQLite.

## Rodar

```bash
cargo run -p ray-task --release
```

Por padrão o app usa o renderer de software (orçamento de RAM < 40 MB); para voltar à GPU: `SLINT_BACKEND=winit-femtovg cargo run -p ray-task --release`.

Requisitos no Fedora: `sudo dnf install fontconfig-devel libxkbcommon-devel wayland-devel` (dependências do backend do Slint).

## Onde ficam as coisas

| O quê | Caminho |
|---|---|
| Banco | `~/.local/share/ray-task/ray-task.db` (backup antes de migrar: `ray-task.db.bak`) |
| Log | `~/.local/state/ray-task/ray-task.log` |
| Lock de instância | `$XDG_RUNTIME_DIR/ray-task.lock` |

## Atalhos

| Atalho | Ação |
|---|---|
| `Ctrl+N` | Nova tarefa na visão atual |
| `Ctrl+Shift+N` | Novo projeto |
| `↑` / `↓` | Navegar entre tarefas |
| `Enter` / `Esc` | Abrir / fechar |
| `Ctrl+Enter` | Concluir / desmarcar |
| `Ctrl+D` | Data |
| `Ctrl+T` | Tag |
| `Delete` | Apagar (com Desfazer) |
| `Ctrl+Z` | Desfazer |
| `Ctrl+1` / `2` / `3` | Hoje / Próximos / Entrada |
| `Ctrl+F` | Filtrar a visão atual |

Clique direito num projeto: renomear, cor, apagar. Clique numa tag aberta: remover, renomear, apagar.

## Testes

```bash
cargo test                                                         # tudo
cargo test -p ray-core --release --test perf -- --ignored --nocapture  # orçamento de performance
```

Fonte Inter © The Inter Project Authors, licença SIL OFL 1.1 (`crates/app-slint/assets/fonts/LICENSE.txt`).
