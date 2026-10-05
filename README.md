<p align="center"><img src="crates/app-slint/assets/icon/ray-task-128.png" alt="ray-task" width="128" height="128"></p>

# ray-task

TODO desktop, local, leve e fluido. Rust + Slint + SQLite.

## Rodar

```bash
cargo run -p ray-task --release
```

Por padrão o app usa o renderer de software (orçamento de RAM < 40 MB); para voltar à GPU: `SLINT_BACKEND=winit-femtovg cargo run -p ray-task --release`.

Requisitos no Fedora: `sudo dnf install fontconfig-devel libxkbcommon-devel wayland-devel` (dependências do backend do Slint).

## Instalar no menu

```bash
cargo build -p ray-task --release
install -Dm755 target/release/ray-task ~/.local/bin/ray-task
for n in 16 32 48 64 128 256 512; do
  install -Dm644 crates/app-slint/assets/icon/ray-task-$n.png \
    ~/.local/share/icons/hicolor/${n}x${n}/apps/ray-task.png
done
install -Dm644 packaging/ray-task.desktop ~/.local/share/applications/ray-task.desktop
```

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
| `Ctrl+D` | Data (no popover: `H` Hoje, `A` Amanhã, `S` Próxima semana, `Delete` Sem data, ou digite a hora) |
| `Ctrl+T` | Tag (`Enter` usa o texto digitado, `Tab` aceita a sugestão) |
| `Delete` | Apagar (com Desfazer) |
| `Ctrl+Z` | Desfazer |
| `Ctrl+1` / `2` / `3` | Hoje / Próximos / Entrada |
| `Ctrl+4` … `Ctrl+9` | Projetos, na ordem da barra lateral |
| `Ctrl+F` | Filtrar a visão atual |

Clique direito num projeto: renomear, cor, apagar. Clique numa tag aberta: remover, renomear, apagar.

## Testes

```bash
cargo test                                                         # tudo
cargo test -p ray-core --release --test perf -- --ignored --nocapture  # orçamento de performance
```

## Licença

MIT — veja [LICENSE](LICENSE).

Fonte Inter © The Inter Project Authors, licença SIL OFL 1.1 (`crates/app-slint/assets/fonts/LICENSE.txt`).
