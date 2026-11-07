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

## Não verificado

- **`scripts/dev-app.sh` no app real (superfície C da skill).** O comando que ele executa foi medido à mão acima, mas o script em si ainda não rodou com o app: o slot do app estava com outra sessão. Entra aqui quando rodar: comando, Trust, árvore, recusa por instância viva, e `pgrep` vazio no fim.
- Windows e macOS Intel: fora do alvo.
