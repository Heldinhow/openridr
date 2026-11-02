# SPEC — correção dos achados 1–4 de `QA-DEVFLOW-2026-11-02.md`

Escopo: achados 1, 2, 3 e 4 (com a persistência do achado 5, que o 4 exige). Achados 6–26 ficam fora. Cada achado tem commit próprio, testes escritos antes da correção e prova no host real.

## 1. Debug rola até a linha de execução

- Quando `currentLine` (frame selecionado) muda para uma linha válida, o `CodeEditor` rola o mínimo necessário para a linha aparecer; se a linha já está visível, nada se move.
- Vale para qualquer origem da linha atual: breakpoint, launch, step into/over/out, Run to Cursor e parada por exceção. Não rouba foco do painel de Debug.
- A linha continua marcada com `.cm-line-current`.

Teste: `tests/e2e/qa-devflow-top4.spec.ts`, bloco 1 (netcoredbg real; skip honesto sem o adaptador). Programa com a linha de parada abaixo da dobra, editor no topo: após pausar, `.cm-line-current` fica dentro do `.cm-scroller` e `scrollTop > 0`.

## 2. Chord `⌘U` funciona com foco no editor

- Com foco no editor, `⌘U` arma o chord (sem chamar `undoSelection`) e a letra seguinte (`L`/`F`/`D`/`R`) executa a ação de testes correspondente.
- A letra do chord nunca chega ao buffer e `⌘U` sozinho não altera o documento.
- Fora do editor o comportamento atual continua.

Teste: bloco 2. Descobrir testes, abrir `OrderTests.cs`, focar o editor, `Meta+u` `l`: o conteúdo do buffer não muda e um run de testes inicia/conclui.

## 3. Atalhos globais têm precedência sobre o `defaultKeymap` do CodeMirror

- O handler global do app roda em fase de captura; quando o app reconhece um atalho (ação ou chord), ele consome o evento antes do CodeMirror.
- `⇧⌘K` abre o diálogo de Push e não apaga a linha; `⌃D` inicia o Debug e não apaga caractere.
- Mesma causa revisada nos demais atalhos globais: `⌘U` (prefixo), `⇧⌘K`, `⌃D` e `⌘G`. `⌘G` fica deliberadamente com o CodeMirror até o Go to Line existir (achado 16, fora do escopo), para não trocar `findNext` por um stub silencioso.
- Atalhos do editor que o app não registra (setas, `⌘Z`, `⌘/`, etc.) continuam no CodeMirror.

Teste: bloco 3. Com foco no editor: `Shift+Meta+k` abre `push-dialog` e preserva o buffer; `Control+d` muda `dap-state` e preserva o buffer (skip honesto sem netcoredbg).

## 4. Solução com vários consoles: toolbar utilizável e Build da solução

- Com 2+ projetos console, `startupProject` cai no último escolhido ou no primeiro candidato da avaliação; Build, Run, Debug e o seletor de Run configuration não ficam desabilitados por ambiguidade.
- A toolbar ganha um select `toolbar-startup` com os projetos console; escolher ali atualiza o startup project (e a configuração ativa, como o select do painel Build).
- `⇧⌘F9` / item "Build Solution" no menu da toolbar / botão `build-solution` no painel Build executam `dotnet build <sln> -c <config> --no-restore` e reportam problemas de todos os projetos.
- Um build de solução bem-sucedido serve para Run/Debug do startup escolhido (mesma configuração/TFM e fontes inalteradas); não força um segundo build.
- A escolha do startup project é persistida por solução no store de run configurations e restaurada ao reabrir.

Teste: bloco 4 com a fixture `fixtures/MultiConsole` (2 consoles). Toolbar com Build/Run/Debug habilitados, `toolbar-startup` lista os dois, Build Solution compila a solução, Run funciona sem rebuild extra, escolher o segundo console e reabrir a solução restaura a escolha. Teste Rust em `crates/host/src/run_configs.rs` para a persistência.

## Fora do escopo

Achados 6–26 (Build só do startup vira Build Solution apenas como ação explícita; Go to Line, abas, histórico, foco dos diálogos, etc.). O `build.start` por projeto continua igual e é o usado por ⌘F9, pela árvore e pelo Run/Debug implícito.
