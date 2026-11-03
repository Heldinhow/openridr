# Features mapeadas

Cada arquivo responde: o que é, como o usuário chega lá, como dirigir (com o harness de `../SKILL.md`) e qual estado final observável prova que funciona. Seletores são `data-testid` (contrato em `docs/implementation-notes.md#testids-estáveis-e2e`); donos no código: `docs/MAPA.md`.

| # | Feature | Superfícies | Specs que já cobrem | Estado da prova (2026-11-03) |
|---|---|---|---|---|
| 1 | [Abrir solution/workspace](open-workspace.md) | A, B | `workspace`, `welcome`, `open-external` | A: passou. B: passou. C: não verificado |
| 2 | [Árvore da solution + editor](solution-tree-editor.md) | A, B | `solution-explorer`, `workspace`, `code`, `medium-editor-*` | A: passou. B: passou. C: não verificado |
| 3 | [Trust por projeto](trust.md) | A, B + teste de propriedade | `trust-prompt`, `status-mode`, `trust-sdk`, `welcome` | A, B e propriedade: passou. C: não verificado |
| 4 | [Abrir pelo Finder/CLI](open-with.md) | A (host), script, C | `open-external` + `scripts/openridr-selfcheck.sh` + testes de `src-tauri` | A, selfcheck e Rust: passou. C: **não verificado** (PERM_DENIED) |
| 5 | [Build / test / debug](build-test-debug.md) | A | `build`, `run`, `tests`, `debugger`, `execution-gates` | build: passou. test/debug: não rodados nesta passada |

Regra de uso: ao tocar uma feature, rode o doctor, depois o menor conjunto de specs da coluna 4 (+ B se a mudança for visual ou sem spec), e registre em `docs/evidence/`. Feature nova de usuário → adicione um arquivo aqui com as quatro seções.
