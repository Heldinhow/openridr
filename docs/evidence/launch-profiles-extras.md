# Launch profiles: navegador, expansão e argv — evidência

Data: 2026-11-07. macOS Apple Silicon, bundle debug `0.3.0-alpha.11` construído neste worktree
(commit `d1f1c36`, `bun run tauri build --debug --bundles app`), `OPENRIDR_HOME` isolado em `<tmp>`,
cópia descartável do fixture `LaunchProfiles` em `<tmp>`. Uma instância do app por vez; interação e
capturas somente com `agent-desktop`.

## App real

- **Perfil `browser`** (`launchBrowser: true`, `launchUrl: "/page"`, `applicationUrl:
  http://localhost:5090`): o Run abriu o navegador padrão na página do fixture
  ([captura](launch-profiles-extras/01-browser.png)) e o log do Run registrou
  `Opened http://localhost:5090/page in the default browser.`
  ([captura](launch-profiles-extras/02-run-log-browser.png)). O endereço no Safari era
  `localhost:5090/page` (lido da árvore de acessibilidade).
- **Perfil `browser-bad`** (`launchUrl: "file:///etc/passwd"`): nada abriu; o log do Run registrou
  `Launch browser skipped: file:///etc/passwd is not an http(s) URL; only http and https are opened.`
  ([captura](launch-profiles-extras/03-browser-skipped.png)) e nenhuma janela nova apareceu.
- **Perfil `browser-off`** (`launchBrowser: false`): o programa subiu e ficou escutando em 5091
  (`LISTENING=http://localhost:5091/`), o log não ganhou nenhuma linha de navegador e nenhuma janela
  nova apareceu ([captura](launch-profiles-extras/04-browser-off.png)).
- **Perfil `expand`**: `%SHELL%` e `$(SHELL)` expandidos no ambiente e na `applicationUrl`
  (`PROFILE_ENV=/bin/zsh`, `EXPANDED=/bin/zsh`,
  `ASPNETCORE_URLS=http://localhost:5084/?shell=/bin/zsh`) e o argv literal
  (`ARGS=%SHELL%|$(SHELL)`) ([captura](launch-profiles-extras/05-expanded-values.png)).
- **Perfil `http`** com `commandLineArgs` `--a "b c" --d "" --e "f\"g"`: o programa recebeu
  `ARGS=--a|b c|--d||--e|f"g` — o mesmo argv que o `dotnet run` real entrega para a mesma string
  ([captura](launch-profiles-extras/06-argv.png)).
- **Debug** com um perfil `browser-debug` (porta 5092, `launchUrl: "/debug-page"`): sessão DAP
  `Running`, o log DAP registrou `Opened http://localhost:5092/debug-page in the default browser.` e
  o Safari ganhou uma segunda aba ([captura](launch-profiles-extras/07-debug-browser.png)). O log DAP
  não virou imagem porque o cabeçalho da sessão mostra o caminho do adaptador com o home do usuário.

Observações da prova:

- O seletor de perfis foi aberto por clique nas coordenadas do disclosure: o `summary` não tem
  `ref_id` no agent-desktop (limitação conhecida), os itens do menu têm. Os perfis foram escolhidos
  pelo menu, como um usuário faria.
- A cópia descartável do fixture troca `HOME` por `SHELL` no perfil `expand` e acrescenta
  `browser-debug`, para a evidência não gravar caminho pessoal; o fixture do repositório mantém
  `HOME` (o teste de host depende dele).
- A janela do Safari aberta pela prova ficou aberta: fechá-la exigiria permissão de automação do
  macOS (o AppleEvent expirou) e o `press cmd+w` não teve foco verificado.

## Testes

- `cargo test -p openridr-host --lib launch` = 26 passed (matriz de argv, expansão, decisão de
  navegador por perfil, composição/validação de URL, argv do abridor, espera por socket real).
- `cargo test -p openridr-host --lib browser::` = 10 passed.
- **Matriz de argv**: 40 casos medidos contra o `dotnet run` real (SDK 8.0.408, macOS ARM64). Falha-antes
  medida com o divisor antigo compilado à parte: **16 dos 40** casos davam argv diferente ou erro.
- **Falha-antes da expansão**: worktree no commit anterior (`8b93f06`) com o mesmo teste de
  `launch_options` → `%HOME%` e `$(HOME)` literais; no commit desta frente, expandidos.
- **Suíte completa** `cargo test -p openridr-host`: na primeira execução, `105 passed; 3 failed` no
  binário `host`; os três são instalação de `csharp-ls` (`tools_install_is_real_verified_and_reused`,
  `lsp_start_requires_tool_sdk_and_current_restore`,
  `lsp_completion_and_definition_answer_from_the_real_session_and_reject_unknown_documents`). Causa
  medida: cada teste baixa o mesmo `.nupkg` de 21,6 MB do nuget.org em paralelo, o download do host
  tem prazo de 120 s e o `curl` do mesmo arquivo levou >120 s nesta rede; rodando cada um sozinho
  passam (88,6 s, 110,2 s, 119,3 s). `tools.rs`/`http.rs`/`nuget.rs` estão idênticos ao `main` e o
  mesmo teste passa no worktree do `main`. Registrado na tabela de
  [test-stability](test-stability.md); não é regressão desta frente.
- `bun run check` = passou; `cargo clippy --workspace --all-targets -- -D warnings` = passou;
  `cargo fmt --all` aplicado; `bun run tauri build --debug --bundles app` = passou.

Não verificado: Windows (o abridor fica fora e é declarado), macOS Intel, `xdg-open` no Linux, e um
app que demora mais de 15 s para aceitar conexão (o prazo esgota e o log diz isso).
