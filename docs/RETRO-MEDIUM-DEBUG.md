# Retro — QA médios de debug, run e build (2026-11-02)

Escopo: d6, d7, d8, d10, d11 e d12 do `QA-DEVFLOW-2026-11-02.md` e b12 (com b21 como
contexto) do `QA-BUGHUNT-2026-11-02.md`. Spec em
[SPEC-MEDIUM-DEBUG](SPEC-MEDIUM-DEBUG.md), tickets em `docs/tickets-medium-debug/`.
Sete commits de ticket + seis commits de correção do review + o commit de spec/tickets.

## O que funcionou

- Teste vermelho antes do código em todos os tickets que mudam comportamento: helper de
  caminho, crash real da `CrashApp` (SIGABRT), descoberta com erro de compilação, hover com
  netcoredbg real, Build de biblioteca pela árvore e rótulo do Build. Os testes falharam pelo
  motivo certo e ficaram verdes depois.
- Reuso em vez de código novo: `build::parse_line`/`record_problem` para os problemas da
  descoberta, `ExitStatusExt::signal` para o crash, `hoverTooltip` do `@codemirror/view` já
  instalado, `paused_context`/`evaluate_in` do DAP para o hover e `workspaceRelative` para
  abas/frames/pilha.
- O review do diff contra o spec pegou um bloqueador real: o teste de integração do host ainda
  exigia que `build.start` recusasse biblioteca. Também pegou o `kind: "test"` habilitado na
  UI e recusado no host, problemas de descoberta stale ainda visíveis, o tom vermelho em Run
  cancelado (SIGTERM), hover sem checagem de geração e o workspace na raiz do filesystem.
  Todos corrigidos com teste/verificação antes da suíte final.

## O que surpreendeu

- Cancelar um Run passou a informar `signal SIGTERM` (verdade do wait status), mas o painel
  pintava isso como erro. Tom de erro agora é só `failed` ou `exited` com sinal/exit ≠ 0.
- `dirOf("/foo.sln")` devolve `""`, que é indistinguível de "sem diretório": abrir um
  workspace na raiz do filesystem voltava a duplicar abas. O helper trata `""` como `/`.
- Os números 7/10/12 têm nomes diferentes no macOS (`SIGEMT`/`SIGBUS`/`SIGSYS`) e no Linux
  (`SIGBUS`/`SIGUSR1`/`SIGUSR2`); a tabela de nomes passou a ser por OS, não uma lista única.
- Projeto de teste é `kind: "test"`, não `library`; habilitar Build só para biblioteca
  deixaria o projeto de testes de fora, que é justamente o caso do achado b12.
- O botão duplicado do b21 (`build-action` no painel e `toolbar-build`) continua: removê-lo
  quebraria contrato de `data-testid` e vários specs. O painel agora diz o escopo
  ("Build startup 'Nome'"), então a duplicação não mente sobre o alvo.

## Review do diff contra o spec

- Sem bloqueadores após a correção. Importantes corrigidos: teste de integração da
  biblioteca, `test` kind, problemas stale/sealed, tom de Run cancelado, geração do hover,
  raiz do workspace e nomes de sinal por OS.
- Nits aplicados: parser da pilha só marca `.cs` com linha ≥ 1; hover ignora palavras de 1
  caractere, keywords e literais; mensagem do debug de teste é exatamente a constante única.
- b21 fica registrado como duplicação intencional com escopo explícito; d10 fica documentado
  como indisponível, sem botão falso.

## Testes realmente executados

- `bun run check`: verde (tsc + cargo check --workspace --all-targets).
- `cargo test --workspace`: 70 testes de integração do host passaram; única falha é a
  conhecida `file_access_is_confined_to_the_workspace` (profundidade do worktree). A falha
  intermitente de avaliação MSBuild não apareceu nesta rodada. Testes unitários do host: 189.
- Suíte E2E completa (`bun run test:e2e`): **221 passed, 5 failed, 1 skipped**.
  - Falhas = conjunto conhecido: `find-usages` (flake de trust), 2 de `run-configs`, 2 de
    `tests.spec` por revisão `:2` em vez de `:1` (flake de ordem/estado). `zz-acceptance` P0
    passou nesta rodada.
  - Specs novos/afetados verdes em isolamento: `medium-debug-paths` (7), crash do `run.spec`,
    descoberta com erro em `tests.spec`, hover e frame com netcoredbg real, `build.spec`,
    `solution-explorer.spec` (5) e `testsuite.spec` (2).

## Não verificado

- App Tauri/WKWebView: toda a prova foi no Chrome sobre a ponte HTTP.
- Windows e macOS Intel; a tabela de sinais do Linux é `cfg` e não roda neste Mac.
- Corrida do hover entre resume e resposta do adaptador (coberta por geração no host, sem
  teste determinístico) e frame de pilha que não termina em `.cs` (só o caso positivo é E2E).
- Debug de teste: indisponível por decisão registrada, não é regressão desta fatia.
