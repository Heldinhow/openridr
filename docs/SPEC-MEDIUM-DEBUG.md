# SPEC — QA médios de debug, run e build (2026-11-02)

Escopo: achados médios de `QA-DEVFLOW-2026-11-02.md` (d6, d7, d8, d10, d11, d12) e
`QA-BUGHUNT-2026-11-02.md` (b12; b21 entra só como contexto do botão duplicado). Fora:
navegação/busca (feitos no PR #10), editor/estado, ferramentas e demais achados.

Fonte de código: `.scratch/fix-medium-debug/explore-notes.md` (mapa por arquivo:linha no HEAD
`ecc2b1a`). Tickets em `docs/tickets-medium-debug/`.

## 1. d11 — frame do debug reusa a aba do arquivo

- `openFile` normaliza o caminho para a forma relativa ao workspace antes de comparar com as
  abas. Helper único em `src/solutionNodes.ts` (`workspaceRelative`), usado também por
  `navigateTo`; abas, `file.read`, save e diagnósticos continuam com caminho relativo.
- Clicar duas vezes no mesmo frame do call stack (ou clicar num frame de um arquivo já aberto)
  mantém **uma** aba, sem `tab-...` duplicado e sem segundo buffer.
- Teste: helper puro + E2E com netcoredbg real (duas execuções do mesmo clique → 1 aba).

## 2. d8 — crash do Run tem destaque, sinal e pilha clicável

- O host passa a informar o sinal de término do processo filho (`signal: Option<i32>` em
  `ProcessStatus`/`ProcessInfo`; nome curto em `RunView.signal`/`Run.signal`, ex. `SIGABRT`).
  Nada de sintetizar `128+N`: quando não há código nem sinal, continua "exit unknown".
- `runStateLabel`: `exited` + sinal → `Run crashed · SIGABRT`; tag de erro no `run-state`.
  `run-exit` mostra `signal SIGABRT` no lugar de `exit unknown`. Exit sem sinal e ≠ 0 também
  usa a tag de erro; exit 0 continua neutro.
- O log do Run (`run-log`) continua `<pre>`, mas linhas no formato
  `   at ... in /caminho/Arquivo.cs:line N` viram botões (`run-log-frame-N`) que abrem o
  arquivo na linha (normalizado pelo item 1).
- Teste: unidade Rust do mapeamento sinal→nome; E2E com fixture `CrashApp` (exceção não
  tratada) provando sinal, destaque e clique na pilha.

## 3. d7 — erro de compilação da descoberta de testes aparece em Problems

- `tests.discover` passa a devolver `problems`/`problemsTruncated` (mesmo `BuildProblem` do
  build), preenchidos pelo mesmo parser de linha do build para stdout e stderr, com dedupe e
  limite. `Start` da descoberta ganha a raiz do workspace para resolver caminhos.
- Problems ganha a seção `tests-problems` (Contagem + linhas `tests-problem-row-N`), clicável
  como `build-problem-row-N`. O log cru continua disponível.
- Teste: E2E com cópia do OrderDesk e erro de sintaxe no projeto de testes; descoberta falha,
  Problems lista o CS, clique abre o arquivo na linha.

## 4. d12 — hover de variável no editor durante o debug

- Novo método de host `debug.hover { expression }` → `DapEvaluation` sem sobrescrever o
  Evaluate do painel (só permitido com sessão `paused`; trust igual a `debug.evaluate`).
- `CodeEditor` ganha `onHoverEvaluate` opcional; quando a sessão está pausada, `hoverTooltip`
  (de `@codemirror/view`, sem dependência nova) avalia a palavra sob o cursor após ~300 ms e
  mostra `tipo valor` ou o erro do adaptador. Fora de pausa, nenhum tooltip é registrado.
- Teste: E2E com netcoredbg real (hover em `person` mostra o valor); sem adaptador, skip
  registrado como "não verificado".

## 5. d10 — debug de teste: decisão registrada, mensagem clara

- Não é viável com a infraestrutura atual: testes rodam num único `dotnet test` (VSTest cria
  o `testhost` sem pid exposto), o DAP só faz `launch` de assembly console avaliado e o
  projeto de teste é biblioteca. `attach`/testhost não existem no host.
- Registrar a decisão em `docs/decisions.md`, apontar a lacuna no `acceptance-matrix.md` e
  trocar a mensagem fixa por: "Debugging tests needs a test-host attach the debugger does not
  support yet. Use Run selected." (mesma em painel, quick list e menu). Nenhum botão é
  habilitado.

## 6. b12 — projeto de biblioteca: Build habilitado, Run/Debug escondidos

- Menu de projeto: `Build` fica habilitado para projeto avaliado com `TargetPath` (inclusive
  biblioteca e projeto de testes); `Run`/`Debug` somem para `kind != "console"` (em vez de
  aparecerem desabilitados). O gate global da toolbar não muda.
- Host: `build.start` com `project` explícito aceita `console` ou `library` avaliada com
  TargetPath; sem `project`, o startup continua exigindo console. Run/Debug continuam
  recusando não-console. `Build Solution` segue separado.
- UI: `startBuild` procura a configuração pedida em todos os projetos avaliados, não só nos
  consoles.
- Teste: E2E constrói `OrderDesk.Core` pela árvore e vê `build-state` sucesso; menu da
  biblioteca não oferece Run/Debug.

## 7. d6 — rótulo do Build diz o que ele constrói

- O botão `build-action` passa a dizer **Build startup (⌘F9)** (ou `Build startup 'Nome'`),
  nunca "Rebuild", porque o comando é um `dotnet build` incremental do startup project.
  Depois de um Build Solution, o rótulo continua "Build startup" e o tooltip aponta
  `Build Solution (⇧⌘F9)` para a solução inteira.
- Toolbar e painel continuam executando `build.start` (⌘F9); `Build Solution` continua em
  `build.solution` (⇧⌘F9). `data-testid`s existentes são preservados; o botão duplicado da
  toolbar (b21) continua, agora com escopo explícito no painel.
- Teste: E2E confere o texto antes/depois de um Build Solution e o tooltip.

## Fora deste spec

- d9 (restaurar abas/cursor/breakpoints), d13 (duplo clique na árvore), d14–d19, b5–b11,
  b13–b20 e b22–b23. Nenhuma dependência nova de UI/runtime.
