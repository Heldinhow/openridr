---
name: verify-openridr
description: Verifica o OpenRidr (IDE C# estilo Rider; Tauri + host Rust + UI React) rodando de verdade. Duas superfícies - UI via Playwright/navegador contra host-http isolado, e o app Tauri real dirigido por agent-desktop no macOS. Use depois de mexer em abrir workspace, árvore/editor, trust, "abrir com" ou build/test/debug, no lugar de revisão adversarial do diff.
---

# verify-openridr

Prova empírica de que a mudança funciona no app de verdade. É o passo (b) do `/code-review` do `AGENTS.md`. Não substitui `bun run check`; complementa.

Alvo: macOS ARM64. Windows e Intel não foram executados: não declare suporte.

## Qual superfície

| Superfície | Quando | Prova |
|---|---|---|
| **A. Playwright** (`tests/e2e`) | mudança de interface pura: só os specs da área, escrevendo/atualizando o spec do cenário; regra de UI/host já coberta por spec | exit code + lista de specs |
| **B. Navegador dirigido por você** (`scripts/web.sh`) | feature nova ou mudada sem spec; precisa de screenshot | snapshot + captura + estado no host/disco |
| **C. App Tauri real** (`scripts/desktop.sh` + `agent-desktop`; receitas em `test-app-agent-desktop`) | o que só existe no shell: menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance, DMG, Finder/Dock/argv, janela, `__TAURI_INTERNALS__` | snapshot de acessibilidade + screenshot |

Comece por A (barata). Use B quando faltar spec. Use C só para o que A/B não alcançam. Se C estiver bloqueada (ver Doctor), registre **"não verificado no app desktop"**: não finja.

## Doctor (somente leitura; rode primeiro, e sempre que algo estranho aparecer)

```sh
sh .agents/skills/verify-openridr/scripts/doctor.sh [caminho/OpenRidr.app]
```

Valida o bundle passado como argumento, ou `$OPENRIDR_APP`, ou `/Applications/OpenRidr.app`; imprime sempre **qual** caminho validou. Checa bun, cargo, `node_modules` (worktree nova não tem: `bun install`), SDK `8.0.408` (as fixtures fixam, `rollForward: disable`), Chrome, portas ocupadas, versão do bundle vs `package.json`, Acessibilidade e Gravação de Tela do agent-desktop. `FAIL` = resolva antes. `note` = leia.

Superfície C exige: **tela do Mac desbloqueada e acordada**, **Acessibilidade concedida** ao app que roda o agent-desktop (senão `accessibility: denied` no `agent-desktop status`; `agent-desktop permissions --guide` abre os Ajustes, ação do usuário) e Gravação de Tela para `screenshot`.

## Launch

**A. Playwright** (o harness já sobe host e Vite sozinho; default 1420 UI / 4317 host, e o app home é derivado da porta da UI):

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en
bunx playwright test tests/e2e/trust-prompt.spec.ts          # um spec; sem Chrome: PLAYWRIGHT_CHANNEL=chromium
```

Portas: default 1420 (UI) / 4317 (host). Sessão em paralelo, ou com `bun run dev`/`host:http` no mesmo worktree, escolhe outro par (faixa 15xx/44xx) e confere com `lsof -nP -iTCP:<porta> -sTCP:LISTEN`; porta ocupada falha na hora pelo `strictPort` do Vite de teste. Cada spec que sobe host próprio usa `hostPort + 1` até `hostPort + 7`, então confira a faixa e deixe 10 portas de host livres acima do seu par. Porta que o Chrome bloqueia (`net::ERR_UNSAFE_PORT`, ex.: 1719, 1720, 1723) é recusada no load do módulo de ambiente, antes de qualquer servidor subir — inclusive quando a faixa derivada toca uma delas, e a mensagem nomeia a porta culpada e o par; escolha outro par. Rode só os specs da área, não a suíte inteira.

Pronto = `N passed` e exit 0. `skip` não é passe: leia o motivo e registre "não verificado".

**B. Navegador próprio** (host-http em 4417 + Vite em 1520, `OPENRIDR_HOME` e cópia da fixture isoladas no `<state>` impresso pelo `up`; default `${TMPDIR:-/tmp}/verify-openridr-<uid>`, override `VERIFY_OPENRIDR_STATE`; portas diferentes das do E2E para coexistirem):

```sh
sh .agents/skills/verify-openridr/scripts/web.sh up      # imprime UI, host, .sln e app home
sh .agents/skills/verify-openridr/scripts/web.sh status
sh .agents/skills/verify-openridr/scripts/web.sh down
```

Pronto = linha `ready: UI http://127.0.0.1:1520 ...`. Recusa subir se 4417/1520 estiverem ocupadas (não dirija a instância de outra pessoa). Trust store **ligado** (comportamento real por projeto).

**C. App real** (binário do bundle com `OPENRIDR_HOME` isolado e `.sln` da fixture em argv, o mesmo caminho do Finder/CLI):

```sh
sh .agents/skills/verify-openridr/scripts/desktop.sh up [caminho/OpenRidr.app]   # argumento > $OPENRIDR_APP > $OPENRIDR_APP_BUNDLE > /Applications/OpenRidr.app; use a cópia sob teste (~/Applications-test/OpenRidr-alpha5/OpenRidr.app)
# workspace próprio (sem copiar a fixture): VERIFY_OPENRIDR_DESKTOP_SOLUTION=/caminho/absoluto/X.sln
agent-desktop list-windows --app OpenRidr
agent-desktop snapshot --app OpenRidr -i --compact
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

Abrir workspace no app real é **por argv** — o mesmo caminho do Finder/CLI, e o que o `desktop.sh` faz. O campo do Welcome **não** recebe digitação (`set-value`/`type` voltam `delivered_unverified` com o valor vazio) e o botão **Open** com o campo vazio abre o seletor nativo do macOS, que deixa o app preso em "Opening…" (o seletor fica esperando, não é a UI). Para o binário de dev, sem bundle, `sh scripts/dev-app.sh <sln>` resolve o par embutido e passa o workspace em argv.

Recusa subir se já houver um `openridr` rodando (bundle de debug e instalado têm o mesmo `CFBundleIdentifier`; a instância pode ser do usuário) e, depois do launch, confere que existe exatamente uma instância. `down` só diz `stopped` com o PID gravado morto e sai não-zero listando o que sobrou. Confira a versão do app (doctor) antes de concluir qualquer coisa: app velho = prova velha. Rebuild: `bun run app:install` ou `bun run tauri build --debug --bundles app`.

## Drive

Seletores estáveis são `data-testid` (contrato; cada feature lista os seus em `features/`).

- **B** (ferramenta `openchamber_web`): `browser.open http://127.0.0.1:1520/` → `browser.type` em `[data-testid="workspace-path-input"]` com o `.sln` impresso por `web.sh up` e `submit:true` → `browser.snapshot` → `browser.click` pelos seletores do snapshot → `browser.capture`. Para estado, consulte o host: `curl -s -X POST http://127.0.0.1:4417/call -H 'content-type: application/json' -d '{"method":"app.state","params":{}}'`.
- **C**: leia `.agents/skills/test-app-agent-desktop/SKILL.md` (receitas, `--headed`, falhas conhecidas). `snapshot` e `click` são headless; `type` e `press` exigem `--headed` **depois** de `agent-desktop focus-window --app OpenRidr` (o `--headed` move o foco do usuário: peça permissão). Sem isso: `post_action_verification`/`delivered_unverified` no `type` e "no verified focused element" no `press`. O app é WKWebView: o conteúdo web aparece na árvore de acessibilidade, mas o `data-testid` não; use papel/nome visível.
- Edite **somente a cópia** da fixture em `<state>/work` (web) ou `<state>-desktop/work` (app real), com o `<state>` impresso pelo `up`; nunca `fixtures/` nem o `~/Library/Application Support/OpenRidr` do usuário.

## Evidence

Prove o caminho de usuário real, não setters internos. Para cada feature capture **ação + estado resultante**, e confira o **efeito colateral** (arquivo em disco, `trusted.json`, resposta do host) além do que se vê na tela.

- Onde guardar: `docs/evidence/<tema>.md` + imagens em `docs/evidence/<tema>/`. Copie as capturas para lá antes do `down` (`.openchamber/` é ignorado pelo git).
- Sem dado pessoal: troque o diretório home por `~` e temporários por `<tmp>`; sem e-mail, sem `/Users/<nome>`. Releia o arquivo com `grep -n "/Users/\|@" <arquivo>` antes do commit.
- O que não rodou entra como **"não verificado"**, com o motivo (permissão, tela bloqueada, ferramenta ausente, Windows/Intel).
- Falhas conhecidas do HEAD (não são suas): E2E `find-usages` (intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts`; Rust `file_access_is_confined_to_the_workspace`. Qualquer outra é sua.

## Cleanup

Derrube só o que você subiu, pelo PID gravado pelos scripts (`web.sh down`, `desktop.sh down`); nunca `pkill`/`killall` por nome. Depois: `lsof -nP -iTCP:1520 -iTCP:4417 -sTCP:LISTEN` deve voltar vazio e `pgrep -x openridr` vazio. Feche a aba que abriu (`browser.close`). O `<state>` e o `<state>.prev` impressos pelo `up` podem ficar (descartáveis) até as evidências estarem copiadas. O Playwright limpa o que ele sobe.

## Features mapeadas

Veja [features/README.md](features/README.md): abrir solution, árvore + editor, trust por projeto, abrir pelo Finder/CLI, build/test/debug.

## Verificação formal

Estado crítico (trust) tem um teste de propriedade guiado por modelo, rodado junto: `cargo test -p openridr-host --test trust_model`. Racional e candidatos em [docs/VERIFICACAO-FORMAL.md](../../../docs/VERIFICACAO-FORMAL.md).

## Manutenção

Para manter este mapa honesto use o skill `maintain-verification-skill`. Em OpenChamber o ciclo é um `schedule.create` com prompt diário; só configure se o usuário pedir.
