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
| **A. Playwright** (`tests/e2e`) | regra de UI/host já coberta por spec; regressão | exit code + lista de specs |
| **B. Navegador dirigido por você** (`scripts/web.sh`) | feature nova ou mudada sem spec; precisa de screenshot | snapshot + captura + estado no host/disco |
| **C. App Tauri real** (`scripts/desktop.sh` + `agent-desktop`) | o que só existe no shell: Finder/Dock/argv, janela, `__TAURI_INTERNALS__` | snapshot de acessibilidade + screenshot |

Comece por A (barata). Use B quando faltar spec. Use C só para o que A/B não alcançam. Se C estiver bloqueada (ver Doctor), registre **"não verificado no app desktop"**: não finja.

## Doctor (somente leitura; rode primeiro, e sempre que algo estranho aparecer)

```sh
sh .agents/skills/verify-openridr/scripts/doctor.sh
```

Checa bun, cargo, `node_modules` (worktree nova não tem: `bun install`), SDK `8.0.408` (as fixtures fixam, `rollForward: disable`), Chrome, portas ocupadas, versão do app instalado vs `package.json`, permissão do agent-desktop. `FAIL` = resolva antes. `note` = leia.

Superfície C exige: **tela do Mac desbloqueada e acordada**, **Acessibilidade concedida** ao app que roda o agent-desktop (senão `PERM_DENIED`; `agent-desktop permissions --guide` abre os Ajustes, ação do usuário) e Gravação de Tela para `screenshot`.

## Launch

**A. Playwright** (o harness já sobe host e Vite sozinho em 4317/4318/1420; não rode junto com `bun run dev`/`host:http`):

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en
bunx playwright test tests/e2e/trust-prompt.spec.ts          # um spec; sem Chrome: PLAYWRIGHT_CHANNEL=chromium
```

Pronto = `N passed` e exit 0. `skip` não é passe: leia o motivo e registre "não verificado".

**B. Navegador próprio** (host-http em 4417 + Vite em 1520, `OPENRIDR_HOME` e cópia da fixture isoladas em `/tmp/verify-openridr`; portas diferentes das do E2E para coexistirem):

```sh
sh .agents/skills/verify-openridr/scripts/web.sh up      # imprime UI, host, .sln e app home
sh .agents/skills/verify-openridr/scripts/web.sh status
sh .agents/skills/verify-openridr/scripts/web.sh down
```

Pronto = linha `ready: UI http://127.0.0.1:1520 ...`. Recusa subir se 4417/1520 estiverem ocupadas (não dirija a instância de outra pessoa). Trust store **ligado** (comportamento real por projeto).

**C. App real** (binário do bundle com `OPENRIDR_HOME` isolado e `.sln` da fixture em argv, o mesmo caminho do Finder/CLI):

```sh
sh .agents/skills/verify-openridr/scripts/desktop.sh up [/caminho/OpenRidr.app]   # padrão /Applications/OpenRidr.app
agent-desktop list-windows --app OpenRidr
agent-desktop snapshot --app OpenRidr -i --compact
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

Recusa subir se já houver um `openridr` rodando (bundle de debug e instalado têm o mesmo `CFBundleIdentifier`; a instância pode ser do usuário). Confira a versão do app (doctor) antes de concluir qualquer coisa: app velho = prova velha. Rebuild: `bun run app:install` ou `bun run tauri build --debug --bundles app`.

## Drive

Seletores estáveis são `data-testid` (contrato; cada feature lista os seus em `features/`).

- **B** (ferramenta `openchamber_web`): `browser.open http://127.0.0.1:1520/` → `browser.type` em `[data-testid="workspace-path-input"]` com o `.sln` impresso por `web.sh up` e `submit:true` → `browser.snapshot` → `browser.click` pelos seletores do snapshot → `browser.capture`. Para estado, consulte o host: `curl -s -X POST http://127.0.0.1:4417/call -H 'content-type: application/json' -d '{"method":"app.state","params":{}}'`.
- **C**: o loop do skill `agent-desktop` (`snapshot --skeleton -i --compact` → `click @ref` → snapshot da mesma região). O app é WKWebView: o conteúdo web aparece na árvore de acessibilidade, mas o `data-testid` não; use papel/nome visível. Prefira ações headless; `--headed` só com permissão do usuário.
- Edite **somente a cópia** da fixture em `/tmp/verify-openridr/work` (ou `/tmp/verify-openridr-desktop/work`), nunca `fixtures/` nem o `~/Library/Application Support/OpenRidr` do usuário.

## Evidence

Prove o caminho de usuário real, não setters internos. Para cada feature capture **ação + estado resultante**, e confira o **efeito colateral** (arquivo em disco, `trusted.json`, resposta do host) além do que se vê na tela.

- Onde guardar: `docs/evidence/<tema>.md` + imagens em `docs/evidence/<tema>/`. Copie as capturas para lá antes do `down` (`.openchamber/` é ignorado pelo git).
- Sem dado pessoal: troque o diretório home por `~` e temporários por `<tmp>`; sem e-mail, sem `/Users/<nome>`. Releia o arquivo com `grep -n "/Users/\|@" <arquivo>` antes do commit.
- O que não rodou entra como **"não verificado"**, com o motivo (permissão, tela bloqueada, ferramenta ausente, Windows/Intel).
- Falhas conhecidas do HEAD (não são suas): E2E `find-usages` (intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts`; Rust `file_access_is_confined_to_the_workspace`. Qualquer outra é sua.

## Cleanup

Derrube só o que você subiu, pelo PID gravado pelos scripts (`web.sh down`, `desktop.sh down`); nunca `pkill`/`killall` por nome. Depois: `lsof -nP -iTCP:1520 -iTCP:4417 -sTCP:LISTEN` deve voltar vazio e `pgrep -x openridr` vazio. Feche a aba que abriu (`browser.close`). `/tmp/verify-openridr*` pode ficar (descartável) até as evidências estarem copiadas. O Playwright limpa o que ele sobe.

## Features mapeadas

Veja [features/README.md](features/README.md): abrir solution, árvore + editor, trust por projeto, abrir pelo Finder/CLI, build/test/debug.

## Verificação formal

Estado crítico (trust) tem um teste de propriedade guiado por modelo, rodado junto: `cargo test -p openridr-host --test trust_model`. Racional e candidatos em [docs/VERIFICACAO-FORMAL.md](../../../docs/VERIFICACAO-FORMAL.md).

## Manutenção

Para manter este mapa honesto use o skill `maintain-verification-skill`. Em OpenChamber o ciclo é um `schedule.create` com prompt diário; só configure se o usuário pedir.
