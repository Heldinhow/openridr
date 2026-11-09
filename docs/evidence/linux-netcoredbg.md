# Evidência — netcoredbg embutido no build do Linux (issue #318)

Data: 2026-11-08. Branch `fix/issue-318-netcoredbg-linux` (base `feat/linux-install`, commit
`9564e1b`). Spec: corpo da issue [#318](https://github.com/Heldinhow/openridr/issues/318).
Decisão: [`../decisions.md`](../decisions.md) (2026-11-08).

Ambiente: **Arch Linux ARM**, kernel `7.2.11-15-1-ARCH`, `aarch64`, Wayland/Hyprland 0.57.1.
Toolchain: `bun` 1.3.9, `cargo` 1.99.0, `dotnet` com SDKs **8.0.408** e **10.0.403** em
`~/.dotnet`. Nenhum `sudo`, nenhuma dependência nova.

## Pin e build

- Assets oficiais da release `3.2.0-1092`, baixados de `github.com/Samsung/netcoredbg` e
  versionados em `vendor/netcoredbg/` (mesmo padrão do zip do macOS):
  - `netcoredbg-linux-arm64-3.2.0-1092.tar.gz`: **3.424.271 bytes**, SHA-256
    `065ff49badec8a695dbea2de6ab6a330c774a191e426a217ab8cc05250627ccb`.
  - `netcoredbg-linux-amd64-3.2.0-1092.tar.gz`: **3.510.717 bytes**, SHA-256
    `080eb3b2d2152465f599d3b33d1ee6e747794e11cc0a3773ec689f5e5f2c5afa`.
  Os dois digests conferem com a tabela da issue (metadata da release).
- `sh scripts/vendor-netcoredbg.sh` no `aarch64`: verifica o SHA-256, valida a listagem e extrai
  para `src-tauri/resources/netcoredbg/` (não versionado). Segunda execução: "is current ...
  nothing to do". O binário extraído responde
  `NET Core debugger 3.2.0-1 (9744e1f, Release)` e tem SHA-256
  `d2ea6a92951c1e7db6554568000c43017f4e5328cbb1157e92d7e9fef7ae198e`.
- `bun run tauri build --no-bundle` a partir do checkout limpo desta worktree (target frio):
  `beforeBuildCommand` do `tauri.linux.conf.json` rodou `bun run build` + `vendor-netcoredbg.sh`
  e **não** chamou `vendor-analysis.sh`; `tauri-build` aceitou `bundle.resources` sem
  `resources/csharp-ls/` e `resources/dotnet/`. O binário final (depois dos achados do
  `/code-review`) é `src-tauri/target/release/openridr`, **23.597.904 bytes**, SHA-256
  `d57e1bbaee6fba6ab2d2b8b1cf569e90afb4bc0e45be656bc08987f062b607cc`; a primeira build da sessão
  (antes do `--version` no snapshot) era `657e2552…` e foi substituída por ela.
- `tests/shell/vendor-netcoredbg.test.sh` sob **dash 0.5.12** (compilado em `/tmp`, porque o Arch
  não empacota dash): 10 checks verdes — pino, reuso, digest divergente, arquivo ausente,
  segmento `..`, caminho absoluto, symlink, arquivo especial, 201 entradas e 201 MiB. Nenhum
  deles instalou o payload rejeitado. O teste chama `vendor_netcoredbg_linux_install` com um
  arquivo controlado; o build sempre passa o arquivo e o digest fixados no script.

## Host

- `cargo check --workspace --all-targets`: ok.
- `cargo test -p openridr-host` com `LC_ALL=C` (o locale `pt_BR` traduz a saída do git e quebra
  três testes de `git::tests` que comparam texto em inglês — falha de ambiente, reproduzida em
  `9564e1b` sem estas mudanças): **487 lib + 10 + 2 + 12 + 89** testes verdes.
- Uma falha pré-existente nesta máquina, reproduzida no commit base `9564e1b` com
  `CARGO_TARGET_DIR` compartilhado: `tools_install_is_real_verified_and_reused` espera
  `Skipping NuGet package signature verification` e o dotnet **10.0.403** escreve
  `dotnet reports skipping package signature verification`. Não é desta frente.
- Testes novos:
  - `debugger::tests::gate_states_are_honest_per_platform`: Linux `x86_64`/`aarch64` com
    `installable: false`, `bundled: true`, código `unsupported_archive`, sem os textos
    antigos; `linux/arm` continua `unsupported_platform`.
  - `debugger::tests::linux_bundled_adapter_is_ready_and_a_missing_or_damaged_one_blocks`:
    `ready_adapter("linux", …)` devolve `linux-arm64`/`linux-amd64` com o adaptador do seam,
    `tool_missing` sem ele e com versão errada.
  - `crates/host/tests/debugger.rs::a_linux_bundle_is_ready_and_a_missing_one_blocks` (só
    Linux): sem adaptador o setup fica bloqueado, `installable: false`, mensagem "nothing is
    downloaded" e `debug.install` recusa com `unsupported_archive`; com o adaptador o estado é
    `ready`, `bundled: true` e `debug.install` recusa com `invalid_state`; removendo o
    executável volta a bloquear.
  - `bundled::tests::the_linux_resource_root_sits_beside_the_binary`: o root é
    `<exe dir>/../lib/<nome>`, nunca o `cwd`.
  - `a_linux_bundle_is_ready_and_a_missing_one_blocks` também cobre o adaptador **danificado**:
    um arquivo que responde `3.1.1` deixa o estado fora de `ready` com "did not verify", e o
    `--version` é cacheado por caminho + mtime + tamanho para o poll de 400 ms não spawnar o
    processo.

## Estado do host: setup, trust e workspace

Bridge de dev (`openridr-host-http`, release) instalado no layout de instalação —
`/tmp/openridr-318/bin/openridr-host-http` + `/tmp/openridr-318/lib/openridr-host-http/netcoredbg` —
com `OPENRIDR_HOME=/tmp/openridr-318/home` e porta reservada **5317**. O host resolveu o
adaptador embutido pelo caminho relativo ao binário:

```json
{
  "setup": { "state": "ready", "runtime": { "state": "found", "version": "10.0.14" } },
  "tools": { "csharp-ls": "ready" },
  "trust": { "decision": "trusted", "granted": true },
  "debug": {
    "adapterState": "ready",
    "bundled": true,
    "installable": false,
    "installedBinary": "/tmp/openridr-318/lib/openridr-host-http/netcoredbg/netcoredbg",
    "sha256": "d2ea6a92951c1e7db6554568000c43017f4e5328cbb1157e92d7e9fef7ae198e",
    "platform": "linux-arm64",
    "availability": "bundled"
  }
}
```

`setup.start` foi de `running/downloading` a `ready/ready` sem baixar netcoredbg: o único
download foi o `csharp-ls` 0.28.0 (fluxo que já existia), e o runtime veio do inventário
(`10.0.14`). `debug.install` não é oferecido (`installable: false`).

## Sessão de debug parando no breakpoint

Workspace `OrderDesk.sln` (cópia de `fixtures/OrderDesk` em `/tmp/openridr-318/OrderDesk`),
breakpoint em `src/OrderDesk.Cli/Program.cs:4`, `debug.start` com
`project: src/OrderDesk.Cli/OrderDesk.Cli.csproj` e `configuration: Debug` (restore e avaliação
antes, como o app faz):

```json
{
  "dap": {
    "state": "paused",
    "stopped": { "allThreadsStopped": true, "reason": "breakpoint", "threadId": 213134 },
    "adapter": {
      "argv": ["/tmp/openridr-318/lib/openridr-host-http/netcoredbg/netcoredbg", "--interpreter=vscode"],
      "binary": "/tmp/openridr-318/lib/openridr-host-http/netcoredbg/netcoredbg",
      "platform": "linux-arm64",
      "version": "3.2.0"
    },
    "breakpoints": [{ "path": "src/OrderDesk.Cli/Program.cs", "line": 4, "verified": true }],
    "threads": [{ "id": 213134, "name": "Main Thread" }],
    "frames": [{ "name": "Program.<Main>$()", "line": 4, "path": "/tmp/openridr-318/OrderDesk/src/OrderDesk.Cli/Program.cs" }],
    "locals": { "scopes": [{ "name": "Locals", "variables": [
      { "name": "args", "type": "string[]", "value": "{string[0]}" },
      { "name": "order", "type": "OrderDesk.Core.Order", "value": "{OrderDesk.Core.Order}" }
    ]}]}
  }
}
```

Árvore de processos no instante da parada (o adaptador é o embutido, não um download):

```text
199464 /tmp/openridr-318/bin/openridr-host-http --port 5317
213123  └─ /tmp/openridr-318/lib/openridr-host-http/netcoredbg/netcoredbg --interpreter=vscode
213134      └─ /home/helder/.dotnet/dotnet .../OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
```

`debug.continue` terminou a sessão com `state: terminated`, `exit: 0`, `exitKnown: true` e a
saída do debuggee `Order A-104: R$ 42,00`.

## App instalado

Binário instalado em `~/.local/bin/openridr` (`d57e1bba…`) e o payload em
`~/.local/lib/openridr/netcoredbg/` — o layout que `resources_root()` resolve no Linux. App
aberto com `OPENRIDR_HOME` isolado e a fixture `/tmp/openridr-318/OrderDesk/OrderDesk.sln`
(home novo, então o setup rodou de verdade):

- **Setup**: a tela "Fixed tools are part of the project setup" mostrou
  `Debug adapter · netcoredbg 3.2.0-1092 — Bundled and verified` com o caminho
  `/home/helder/.local/lib/openridr/netcoredbg/netcoredbg`, `Analysis tool · csharp-ls 0.28.0`
  baixando (o único download) e `Analysis runtime · .NET 10 — Found 10.0.14`. Nenhum download de
  netcoredbg; `installable: false`.
- **Trust**: o prompt "Trust OrderDesk.sln?" apareceu depois do setup; a decisão gravou
  `{"<fixture>": "trusted"}` em `$OPENRIDR_HOME/trusted.json` e o workspace abriu com
  "Trusted mode" e a Solution Explorer com os 3 projetos.
- **Debug**: com o breakpoint em `src/OrderDesk.Cli/Program.cs:4` (F9 na linha) e F5, o painel
  Debug do app instalado mostrou `Paused · breakpoint · 1 frame(s) · 1 thread(s)`, a thread
  `Main Thread (259931)`, o frame `Program.<Main>$()` em `Program.cs:4` e os locais
  `args = {string[0]}` e `order = OrderDesk.Core.Order`. A árvore de processos no instante da
  parada era `openridr (259422) → ~/.local/lib/openridr/netcoredbg/netcoredbg --interpreter=vscode
  (259920) → dotnet .../OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll (259931)`. F5 de novo
  terminou a sessão (`Debug Terminated`, saída do debuggee preservada no painel).

## Não verificado

- **macOS**: a suíte de CI roda os testes de fronteira do macOS; esta sessão só executou o host
  em Linux. O comportamento do macOS foi preservado por construção (`gate.bundled` só é
  verdadeiro lá com o opt-in experimental), mas não foi executado aqui.
- **`x86_64`**: o asset `linux-amd64` tem o mesmo pino e o mesmo caminho de extração, mas não
  houve build nem execução em x86_64.
- **`.deb`/AppImage/updater**: fora do escopo da issue; o build usado foi `--no-bundle`.
