# ray-task — Design Spec

- **Data:** 2026-10-05
- **Status:** aguardando revisão
- **Fase:** A (UI em Slint). Fase B futura: substituir a UI por um mini-framework próprio sobre GPU.

## 1. Objetivo

App de TODO **desktop**, **local**, **leve e fluido**, com UI limpa inspirada em iOS/macOS. Referências de leveza: Todour e Planify. Uso pessoal, um único computador, sem conta e sem rede.

### Critérios de sucesso

- Abertura a frio < 200 ms.
- RAM < 40 MB com 5.000 tarefas.
- Filtrar/montar qualquer visão < 2 ms com 5.000 tarefas.
- Nenhuma interação espera disco (escritas assíncronas).
- Todo o fluxo diário possível só com teclado.

### Requisitos (do usuário)

- Entidades: projeto, tarefa (título, descrição, data), tag.
- N projetos → N tarefas por projeto → N tags por tarefa.
- Tags pertencem a um projeto (isoladas por projeto).
- Simples, rápido, fluido, bonito.

## 2. Stack

- **Rust** (workspace Cargo) + **Slint 1.x** com estilo `cupertino`.
- **SQLite** via `rusqlite` (feature `bundled`).
- `tracing` + `tracing-appender` para log em arquivo.
- Fonte **Inter** embutida no binário (SF Pro não é redistribuível).
- Alvo principal: Linux (Fedora). Nada específico de plataforma é usado, mas só Linux é testado na v1.

Motivos: Slint é estável (semver 1.x), leve, GPU, com animações declarativas e estilo cupertino pronto; o usuário tem Rust básico, e Slint mantém o Rust restrito à lógica. GPUI foi descartado (API instável, sem docs, difícil com Rust básico). Tauri foi descartado (WebKitGTK no Linux prejudica fluidez).

## 3. Arquitetura

```
ray-task/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ core/                   # domínio + persistência; ZERO dependência de UI
│  │  ├─ src/model.rs         # Project, Task, Tag, IDs, DueDate
│  │  ├─ src/store.rs         # estado em memória, regras, visões, undo
│  │  ├─ src/clock.rs         # trait Clock (injeção de data/hora; testes usam relógio fixo)
│  │  ├─ src/db.rs            # SQLite: abrir, migrations, CRUD
│  │  └─ src/writer.rs        # thread de escrita em background
│  └─ app-slint/              # UI
│     ├─ ui/*.slint           # componentes visuais
│     └─ src/main.rs          # liga Store ⇄ modelos Slint, atalhos, timer
└─ docs/
```

### Fluxo de dados

1. **Abertura:** adquire o lockfile → abre o SQLite → roda as migrations → carrega tudo para o `Store` em memória → mostra a janela.
2. **Leitura:** visões, contadores e filtros são calculados a partir da memória, nunca do disco.
3. **Escrita:** a UI chama um método do `Store` (ex.: `complete_task(id)`). O `Store` valida, atualiza a memória, devolve o novo estado à UI e envia um comando (`WriteOp`) por canal para a thread `writer`, que grava no SQLite em transação.
4. **Falha de escrita:** o `writer` reporta pelo canal de volta e a UI mostra um toast (ver §7).

O `Store` é a única API pública consumida pela UI. A fase B troca `app-slint` por `app-gpu` sem alterar `core`.

## 4. Modelo de dados

### Esquema SQLite (v1)

```sql
CREATE TABLE projects (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL,
  color       TEXT    NOT NULL,          -- hex, ex. '#0A84FF'
  sort_order  INTEGER NOT NULL,
  created_at  TEXT    NOT NULL           -- ISO-8601 UTC
);

CREATE TABLE tasks (
  id            INTEGER PRIMARY KEY,
  project_id    INTEGER NULL REFERENCES projects(id) ON DELETE CASCADE, -- NULL = Entrada
  title         TEXT    NOT NULL CHECK (length(trim(title)) > 0),
  notes         TEXT    NOT NULL DEFAULT '',
  due_date      TEXT    NULL,            -- 'YYYY-MM-DD' (data local)
  due_time      TEXT    NULL,            -- 'HH:MM' (hora local); exige due_date
  completed_at  TEXT    NULL,            -- ISO-8601 UTC
  sort_order    INTEGER NOT NULL,
  created_at    TEXT    NOT NULL,
  updated_at    TEXT    NOT NULL,
  CHECK (due_time IS NULL OR due_date IS NOT NULL)
);

CREATE TABLE tags (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name        TEXT    NOT NULL COLLATE NOCASE,
  color       TEXT    NOT NULL,
  UNIQUE (project_id, name)
);

CREATE TABLE task_tags (
  task_id  INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (task_id, tag_id)
);
```

- `PRAGMA foreign_keys = ON`, `PRAGMA journal_mode = WAL`.
- **Trigger** `BEFORE INSERT ON task_tags`: aborta se `tags.project_id` for diferente de `tasks.project_id` (inclusive se a tarefa estiver na Entrada).
- Versão do schema em `PRAGMA user_version`.
- Local do arquivo: `$XDG_DATA_HOME/ray-task/ray-task.db` (padrão `~/.local/share/ray-task/ray-task.db`).

### Regras de domínio (aplicadas no `Store`, reforçadas no banco)

| Regra | Comportamento |
|---|---|
| Tag ↔ projeto | Uma tarefa só recebe tags do próprio projeto. Tarefa na Entrada não tem tags. |
| Mover tarefa de projeto | Remove todas as tags dela. A UI avisa antes se a tarefa tiver tags. |
| Mover tarefa para Entrada | Idem: remove as tags. |
| Apagar projeto | Apaga as tarefas e tags dele (cascade), após confirmação. |
| Concluir / desmarcar | Define / limpa `completed_at`. |
| Título | Obrigatório, com espaços nas pontas removidos. Tarefa nova com título vazio é descartada. |
| Nome de tag | Espaços removidos; único por projeto sem diferenciar maiúsculas. |
| Hora sem data | Inválido: limpar a data limpa a hora. |

## 5. Visões

| Visão | Conteúdo | Ordenação |
|---|---|---|
| **Hoje** | Não concluídas com `due_date <= hoje` (atrasadas incluídas, destacadas) | atrasadas primeiro; depois por hora (sem hora por último); depois `sort_order` |
| **Próximos** | Não concluídas com `due_date > hoje`, agrupadas por dia | data, hora, `sort_order` |
| **Entrada** | `project_id IS NULL` | não concluídas por `sort_order`; concluídas ocultas |
| **Projeto X** | Tarefas do projeto | não concluídas por `sort_order`; seção "Mostrar concluídas" recolhida (mais recentes primeiro) |

"Hoje" é a data local, vinda do `Clock`.

**Onde nasce uma tarefa nova** (`Ctrl+N`):
- Em Hoje: na Entrada, com prazo hoje.
- Em Próximos: na Entrada, com prazo amanhã.
- Na Entrada: na Entrada, sem prazo.
- Em um projeto: no projeto, sem prazo.

Contadores na sidebar mostram as não concluídas de cada visão/projeto.

## 6. UI e interação

**Referência visual aprovada:** `docs/superpowers/mockups/2026-10-05-ui-preview.html` (mockup HTML interativo, aprovado em 2026-10-05). Cores, espaçamentos, raios e timings de animação da implementação seguem esse mockup.

### Tokens de cor (do mockup)

| Token | Claro | Escuro |
|---|---|---|
| fundo | `#FFFFFF` | `#1C1C1E` |
| sidebar | `#F5F5F7` | `#242426` |
| texto | `#1D1D1F` | `#F5F5F7` |
| texto secundário | `#6E6E73` | `#A1A1A6` |
| linhas | `#E5E5EA` | `#3A3A3C` |
| chip | `#F0F0F3` | `#2E2E31` |
| seleção sidebar | `#DCDCE1` | `#3A3A3D` |
| destaque | `#007AFF` | `#0A84FF` |
| vermelho (atrasada) | `#D70015` | `#FF6961` |
| laranja (em breve) | `#C25E00` | `#FFB340` |

Raios: linha/pill 8–10 px, popover 14 px, janela 12 px. Ícones das visões: Hoje laranja `#FF9F0A`, Próximos vermelho `#FF453A`, Entrada azul `#0A84FF`.

### Layout

- Janela padrão 960×640, mínima 640×420.
- **Sidebar** (220 px): Hoje, Próximos, Entrada, seção PROJETOS (bolinha de cor + nome + contador), "+ Projeto" no rodapé. Fundo um tom abaixo do conteúdo.
- **Conteúdo:** título da visão + data atual; lista de tarefas; "+ Nova tarefa" ao final.
- **Linha de tarefa:** checkbox circular, título, chips de tags, hora/indicadores à direita.
- **Edição inline** (estilo Things): Enter ou clique expande a tarefa no lugar, mostrando título editável, notas (multilinha), data/hora e tags. Esc fecha e salva. Sem modais nem painel lateral.
- **Popover de data:** atalhos Hoje / Amanhã / Próx. semana / Sem data, mini-calendário, campo de hora opcional.
- **Tags:** chips com autocomplete das tags do projeto; Enter cria tag nova; Backspace em campo vazio remove o último chip.
- **Tema:** segue o sistema (claro/escuro); destaque azul Apple (`#007AFF` claro / `#0A84FF` escuro); fonte Inter.
- **Gerenciar projeto:** menu de contexto (clique direito) na sidebar com Renomear, Cor e Apagar (com confirmação que mostra quantas tarefas serão apagadas). Tags são renomeadas/apagadas pelo mesmo menu dentro do chip.

### Animações

| Ação | Animação |
|---|---|
| Concluir | check preenche (150 ms); após ~600 ms a linha colapsa com fade (200 ms). Desfazer possível nesse intervalo e pelo toast. |
| Expandir / fechar | altura + opacidade, 200 ms ease-out |
| Nova tarefa | linha cresce de altura 0 (180 ms) |
| Trocar visão | crossfade 120 ms; destaque da sidebar desliza até o item |
| Apagar | linha colapsa; toast "Tarefa apagada · Desfazer" por 5 s |
| Tarefa vence (app aberto) | um pulso suave na linha, uma única vez |

### Indicadores de horário (sem notificações do sistema)

- Tarefa com hora, de hoje, faltando ≤ 60 min: hora em **laranja** + "em N min".
- Hora passada: hora em **vermelho** + "há N min" / "há N h".
- Tarefa com data passada (atrasada): data em vermelho.
- Contador de **Hoje** na sidebar fica vermelho se houver qualquer atrasada.
- Um timer alinhado à virada de cada minuto recalcula os indicadores; na virada do dia, recalcula as visões Hoje/Próximos.

### Atalhos

| Atalho | Ação |
|---|---|
| `Ctrl+N` | Nova tarefa na visão atual |
| `Ctrl+Shift+N` | Novo projeto |
| `↑` / `↓` | Navegar entre tarefas |
| `Enter` / `Esc` | Expandir-editar / fechar |
| `Ctrl+Enter` | Concluir / desmarcar |
| `Ctrl+D` | Abrir popover de data |
| `Ctrl+T` | Adicionar tag |
| `Delete` | Apagar tarefa (com Desfazer) |
| `Ctrl+Z` | Desfazer última ação destrutiva (apagar, concluir, mover) |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Hoje / Próximos / Entrada |
| `Ctrl+F` | Filtrar a visão atual por texto (título + notas) |

**Desfazer:** o `Store` guarda uma pilha curta (últimas 20 ações) de inversas para apagar, concluir e mover. Desfazer apagar reinsere com o mesmo id, tags e posição.

## 7. Tratamento de erros

| Situação | Comportamento |
|---|---|
| Banco não abre (corrompido / permissão) | Tela de erro com caminho e motivo. Nunca abre vazio silenciosamente. |
| Falha ao gravar | Memória mantém o estado; toast "Falha ao salvar · Tentar de novo"; log do erro. A operação fica na fila do writer até sucesso ou fechamento. |
| Migration | Cópia `ray-task.db.bak` antes de migrar; falha na migration → tela de erro, banco original intacto. |
| Segunda instância | Lockfile em `$XDG_RUNTIME_DIR/ray-task.lock` (fallback: diretório de dados); a segunda instância exibe "ray-task já está aberto" e sai. |
| Fechar com escritas pendentes | A janela espera o writer drenar a fila (timeout 2 s) antes de sair. |

Log: `$XDG_STATE_HOME/ray-task/ray-task.log` (padrão `~/.local/state/ray-task/`), via `tracing`, nível `info`.

## 8. Testes

- **core — comportamento (maior parte):** tag de outro projeto rejeitada; mover limpa tags; apagar projeto faz cascade; concluir/desmarcar; desfazer de apagar/concluir/mover; regras de título e tag; montagem de Hoje/Próximos/Entrada/Projeto com `Clock` fixo (inclui atrasadas, virada do dia, tarefas com e sem hora); onde nasce a tarefa nova em cada visão.
- **db:** SQLite em memória; roundtrip de todas as entidades; migration de banco vazio até a versão atual; trigger tag↔projeto; cascades.
- **writer:** falha simulada de escrita gera evento de erro e mantém a operação na fila.
- **UI:** smoke tests com o backend de testes do Slint (abrir, criar tarefa, concluir) + checklist manual de animações e atalhos.
- **Performance:** benchmark com 5.000 tarefas checando os critérios da §1 (abertura, RAM, montagem de visão).

## 9. Fora do escopo (v1)

Sync / multi-dispositivo, recorrência, celular, notificações do sistema / lembretes em background, subtarefas, prioridades, import/export, busca global entre projetos, Windows/macOS testados, reordenação por arrastar (a ordem segue `sort_order` de criação; reordenação via teclado também fica fora).

## 10. Fase B (registro, não implementar)

Substituir `app-slint` por uma UI própria sobre `winit` + `wgpu` + `taffy` + `cosmic-text`/`glyphon` + `vello`, com sistema de springs próprio. Pré-requisito garantido por esta spec: `core` sem dependência de UI e `Store` como API única.
