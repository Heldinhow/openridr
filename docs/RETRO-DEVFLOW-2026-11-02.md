# RETRO — correção dos achados 1–4 do QA de devflow (2026-11-02)

## O que foi entregue

- **Achado 1:** `CodeEditor` rola até a linha atual do debug quando ela muda e está fora da viewport, sem roubar foco. Um só efeito cobre breakpoint, launch, step e exceção.
- **Achados 2/3:** o keymap global passou para captura e consome chords/ações antes do CodeMirror. `⌘U`+letra, `⇧⌘K` e `⌃D` funcionam com foco no editor e não alteram o buffer. `⌘G` continua no CodeMirror até o Go to Line sair do stub (achado 16).
- **Achados 4/5:** com vários consoles o startup project cai na escolha persistida ou no primeiro candidato; a toolbar ganhou `toolbar-startup`; a escolha é gravada por solução. `build.solution` compila o `.sln`, publica problemas de todos os projetos e o build resultante é reutilizado por Run/Debug. `build.start` (⌘F9) continua por projeto.

## Evidência real

- `bun run check` verde em todos os commits.
- `cargo test --workspace`: 186 unit + 9 debugger + 71 host; **2 falhas pré-existentes** reproduzidas no commit base `cb8ae7d` (ver abaixo).
- `tests/e2e/qa-devflow-top4.spec.ts`: 4 blocos com dotnet, csharp-ls/netcoredbg reais quando presentes; o bloco de Debug faz `skip` honesto sem o adaptador.
- `tests/e2e/actions.spec.ts`: 6/6, incluindo a nova precedência de override do usuário sobre o CodeMirror.
- Suíte E2E completa (rodada final): **184 passed, 6 failed, 1 skipped (14,5 min)**. As 6 falhas restantes são as pré-existentes abaixo, reproduzidas no base; `actions.spec.ts` e os 4 blocos novos passaram.

## Falhas pré-existentes (não corrigidas, fora do escopo)

Reproduzidas no commit base `cb8ae7d` com o mesmo ambiente:

- `cargo test --test host file_access_is_confined_to_the_workspace`: o caminho `../../../../../../etc/hosts` cai fora do `/etc/hosts` neste checkout profundo e vira `not_found` em vez de `outside_workspace`.
- `cargo test --test host evaluation_returns_the_real_msbuild_model_for_every_project`: a fixture `OrderDesk` sem `obj/` avaliada como `not_restored` (o teste espera `restored` sem rodar restore).
- `tests/e2e/find-usages.spec.ts`: `lsp-state` fica em "Partial coverage" em vez de "Ready".
- `tests/e2e/run-configs.spec.ts` (2 casos de rebuild falho): o build antigo continua "succeeded" porque o teste deixa o buffer sujo e não decide salvar; falha igual na base.
- `tests/e2e/zz-acceptance.spec.ts` (jornada P0): `debug.install` devolve `downloading` em vez de `ready` na cópia do adaptador; falha igual na base.
- `tests/e2e/tests.spec.ts` (2 casos de discovery/run) falharam na rodada completa por revisão `:2` em vez de `:1`, mas passaram isolados na mesma árvore; classificado como flake de ordem/estado, não regressão.

## Não verificado

- App Tauri/WKWebView empacotado: toda a prova de UI foi no Chrome sobre a ponte HTTP de dev.
- Windows e macOS Intel.
- Debug real reutilizando um build de solução em sessão longa (o caminho host foi alterado; o E2E cobre Run, não DAP).
- Diálogo de buffers sujos do Build Solution em todas as combinações de escolha.
- `build.solution` em solução multi-TFM: o comando envia `-p:TargetFramework` da avaliação e pode falhar em projetos que não compartilhem o TFM.

## Lições

- Atalho global que precisa vencer o editor tem de ser capturado antes do CodeMirror; reagir em bubble é sempre tarde quando o `defaultKeymap` já chamou `preventDefault`.
- Persistência por solução precisa de uma operação read-modify-write única no host; dois comandos separados (configs e startup) perdem atualização se rodarem concorrentes.
- Reutilizar um build de solução em Run/Debug exige distinguir escopo (`solution`) de identidade de projeto; comparar `project.path` sozinho força rebuild desnecessário.
- Teste de regressão de debug depende de ferramenta real e precisa de `skip` explícito; a suíte não pode transformar "não instalado" em passe.
