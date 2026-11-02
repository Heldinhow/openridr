# Retro — QA médios de navegação e busca (2026-11-02)

Escopo: achados b5, b6, b7, b8, b9, b16 e b19 do `QA-BUGHUNT-2026-11-02.md` e d14–d18 do
`QA-DEVFLOW-2026-11-02.md`. Spec em [SPEC-MEDIUM-NAV-SEARCH](SPEC-MEDIUM-NAV-SEARCH.md),
tickets em `docs/tickets-medium-nav/`. Doze commits de código + o commit de spec/tickets.

## O que funcionou

- Teste vermelho antes do código em todos os tickets: os 27 casos novos de
  `tests/e2e/medium-nav-*.spec.ts` falharam pelo motivo certo antes de cada correção e
  passaram na suíte completa final.
- Reuso em vez de biblioteca nova: `gotoLine` do `@codemirror/search`, `search.text` do host,
  `copyLineDown`/`deleteLine` do `@codemirror/commands`, o modal de Git existente e o
  `search.text` com `caseSensitive` que já existia.
- O review do diff contra o spec pegou dois problemas reais antes da suíte final: o menu de
  projeto passava o `.csproj` como escopo (Find in Files voltava vazio) e o `FindAction` não
  aplicava a aba pedida quando já estava montado. Ambos ganharam teste e correção.
- `⌘T` saiu só do perfil Rider; o teste de `actions.spec.ts` foi atualizado e o title do botão
  da toolbar passou a dizer "Double Shift" em vez de ficar vazio.

## O que surpreendeu

- Remover `⌘T` quebrou `shell.spec.ts`, que afirmava `Cmd+T` no title do botão de busca. Era um
  contrato escondido no teste, não no registry.
- Find Usages e rename/code actions disputam a mesma classe `.editor-popup`. Manter a faixa
  inferior em 220 px só para Find Usages exigiu escopar a regra
  `:has(.editor-popup:not(.find-usages))`; rename/code actions continuam reduzindo a faixa a
  96 px para caber em 1100×700.
- `csharp-ls.spec.ts` "problems" e `find-usages.spec.ts` falham de forma intermitente na suíte
  completa e passam/alternam em isolamento; o mesmo vale para `restore.spec.ts`, que falhou no
  cleanup (`ENOTEMPTY`) e não no produto.
- O teste de rename em 1100×700 ainda media os botões Apply/Cancel dentro do preview depois de
  a faixa inferior voltar a 220 px; foi preciso devolver a redução só para os previews de
  rename/code actions.

## Review do diff contra o spec

- Sem bloqueadores. Dois importantes corrigidos: escopo de projeto no Find in Files e aba
  inicial do `FindAction`; um nit corrigido: Recent Files desabilitado sem histórico.
- O spec pedia Find in Files com escopo e Match case, não regex/máscara; o host não suporta
  regex/máscara e isso ficou registrado em `decisions.md`.
- Format/Organize Usings não entraram porque o host não expõe `textDocument/formatting` nem
  `source.organizeImports`; a limitação está em `decisions.md`.
- A regra de notificações absolutas não move abas/editor e o drawer não rola na horizontal nos
  dois tamanhos testados.

## Verificação real

- `bun run check`: verde.
- `cargo test --workspace`: 186 + 9 + 70 verdes; 1 falha conhecida
  (`file_access_is_confined_to_the_workspace`, profundidade de caminho). A falha intermitente
  de `evaluation_returns_the_real_msbuild_model_for_every_project` passou nas rodadas finais.
- `bun run test:e2e` (OPENRIDR_E2E_HOME em `.scratch/e2e-home`, Chrome real): **210 passed,
  7 failed, 1 skipped**. Falhas: `find-usages` (intermitente; falha no trust), `restore`
  (cleanup `ENOTEMPTY`), 2 de `run-configs` (baseline), 2 de `tests` (baseline do ambiente) e
  `zz-acceptance` P0 (baseline). Nenhuma falha nos specs novos; `csharp-ls` "problems" e
  "rename previews" passaram na rodada final e em isolamento.
- Commit local por achado; sem push, PR ou release. `~/Library/Application Support/OpenRidr`
  não foi tocado.

## Dívida / não feito

- Find in Files sem regex e sem máscara de arquivo; Recent Files é lista da sessão (não
  persiste) e o caminho `dialogBusy` dos diálogos Git não tem cobertura E2E.
- Format/Organize Usings dependem de uma decisão de host (novas requisições LSP), fora desta
  fatia.
- App Tauri/WKWebView, Windows e macOS Intel não foram executados; a prova é Chrome sobre a
  ponte HTTP de dev.
- `find-usages`, `restore`, `run-configs`, `tests` e `zz-acceptance` seguem como falhas
  intermitentes/ambiente; não foram corrigidos aqui.
