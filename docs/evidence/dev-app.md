# Evidência — app de dev em um comando e abertura por argv (issue #91)

Worktree `chore/issue-91-dev-app-one-command`, macOS 27.2 arm64, 2026-11-07. Spec: [SPEC-DEV-APP](../SPEC-DEV-APP.md). Tickets: #172 (script), #173 (docs).

## O que estava quebrado (medido antes de mexer)

Nesta worktree, `src-tauri/resources/` só tem `dotnet.entitlements` e `netcoredbg/netcoredbg.entitlements`; `vendor/` tem o zip do netcoredbg e **não** tem `dotnet-runtime`/`csharp-ls` (baixam da rede).

```sh
$ bunx tauri dev
   Compiling openridr v0.3.0-alpha.11 (...)
   resource path `resources/dotnet` doesn't exist
warning: build failed, waiting for other jobs to finish...
```

O erro é do `tauri-build` (build script), não do CLI: `bundle.resources` do `tauri.conf.json` é validado no compile. Mesmo com os resources no lugar, o binário de dev não está dentro de um `.app`, então `resources_root()` (`<exe>/../Resources`) não existe e o par só aparece pelos três `OPENRIDR_BUNDLED_*`.

## A flag que remove o requisito de resources

`TAURI_CONFIG` passa por merge patch (RFC 7386) no `tauri-build`, então `null` remove a chave e `{}` não faz nada:

```sh
$ bunx tauri dev --config '{"bundle":{"resources":{}}}'
   resource path `resources/dotnet` doesn't exist          # objeto vazio é no-op

$ bunx tauri dev --config '{"bundle":{"resources":null}}'
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.20s
   Running `target/debug/openridr /tmp/issue91-dev/work/OrderDesk/OrderDesk.sln --no-default-features --color always --`
```

Com os três overrides apontando o `Contents/Resources` de um app instalado, o app abriu o `.sln` do argv, mostrou o Trust e, depois de **Trust project**, carregou a árvore da solution (`OrderDesk · 3 projects`), com `hostroot` composto no `OPENRIDR_HOME` isolado — o par embutido veio dos overrides, não de download. Instância encerrada por PID; `pgrep -x openridr` vazio.

## Self-check do script (roda sem app, sem rede, sem compilar)

```sh
$ sh scripts/dev-app-selfcheck.sh
dev-app self-check: ok
$ dash scripts/dev-app-selfcheck.sh
dev-app self-check: ok
```

Cobre, numa raiz falsa com `bun`, `pgrep` e `tauri` falsos: par vendorizado (exports + `dev --config {"bundle":{"resources":null}} -- <workspace absoluto>`), workspace com espaços como **um** argumento, sem workspace (sem `--`), fonte `OPENRIDR_APP` com a versão do `Info.plist` impressa, fonte relativa resolvida para absoluta, par incompleto nomeando o caminho que falta, runtime `10.0.14-preview` e `10.invalid` recusados (o host só aceita 10.x estável), ausência de par com as duas saídas, overrides parciais recusados, instância viva recusada, caminho inexistente (exit 1) e extensão não suportada (exit 2), `bun`/`node_modules` ausentes pedindo a correção.

`shellcheck -s sh` nos dois scripts: só `SC1007` no idioma `CDPATH= cd -- …`, o mesmo que `scripts/openridr` e `install-app.sh` já produzem.

## O script no app real (superfície C, `agent-desktop`; sem Playwright)

Worktree sem `src-tauri/resources`, `OPENRIDR_HOME` isolado em `/tmp/verify-openridr-501-issue91/home`, cópia da fixture em `.../work/OrderDesk`:

```sh
$ OPENRIDR_HOME=/tmp/verify-openridr-501-issue91/home sh scripts/dev-app.sh /tmp/verify-openridr-501-issue91/work/OrderDesk/OrderDesk.sln
dev-app.sh: pair from /Applications/OpenRidr.app 0.3.0-alpha.9
dev-app.sh: workspace /private/tmp/verify-openridr-501-issue91/work/OrderDesk/OrderDesk.sln
...
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
   Running `target/debug/openridr /private/tmp/verify-openridr-501-issue91/work/OrderDesk/OrderDesk.sln --no-default-features --color always --`
```

- `agent-desktop snapshot` mostrou `dialog: Trust “OrderDesk.sln”?`; o clique em **Trust project** levou à árvore carregada (`OrderDesk · 3 projects`, `src · 2 projects`, `tests · 1 project`) — captura em [dev-app/dev-app-workspace.png](dev-app/dev-app-workspace.png).
- Estado no disco: `trusted.json` com o `.sln`, `hostroot` composto, e **sem** `tools/` no home — nada foi baixado, o par veio dos três overrides.
- Guard de instância: com o app vivo, rodar o script de novo saiu com `exit 1` e `dev-app.sh: an OpenRidr is already running; close it first.`
- Encerrado pelo PID do app (72727); `pgrep -x openridr` vazio, 1420/4317 livres.

## Não verificado

- Windows e macOS Intel: fora do alvo.
- O caminho "par vendorizado" no app real: o self-check cobre os exports e o argv, e o app real foi provado com o par do `/Applications` (o caso que a issue descreve). Vendorizar exige rede e não muda o que o app recebe.
