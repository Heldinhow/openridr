# Host auxiliar do E2E em porta efêmera (issue #246)

Medição em macOS Apple Silicon (arm64), macOS 27.2, 2026-11-08, `PLAYWRIGHT_CHANNEL=chromium` (esta
máquina não tem Chrome), Playwright 1.64.1, bun 1.3.9, SDK **10.0.403 efêmero** em
`/tmp/openridr-dotnet10` (`PATH` + `DOTNET_ROOT`; `~/.dotnet` não foi tocado), `workers: 1`, branch
`test/issue-246-ephemeral-port`. Logs e artefatos fora do repositório (`/tmp`).

## O que foi medido

1. `openridr-host-http --port 0` imprime a porta que o kernel concedeu — teste Rust novo, **vermelho**
   antes da correção.
2. O helper lê essa porta da saída do binário e continua recusando um `/health` de `pid` estranho.
3. Nenhum spec declara mais `hostPort + N`.
4. Duas sessões **em paralelo no mesmo worktree**, cada uma com seu par de portas, verdes.
5. Os sete specs de host próprio verdes (o `debug-advanced` faz skip: sem o netcoredbg real nesta
   máquina).

## (a) O binário: 2/2, e o vermelho antes

```sh
cargo test -p openridr-host-http --test ephemeral_port
```

Antes da correção (`println!` com o endereço **pedido**):

```text
test port_zero_prints_the_port_the_kernel_granted ... FAILED
assertion `left != right` failed: the bridge printed the requested port: openridr-host-http listening on http://127.0.0.1:0
```

Depois (`server.server_addr()`):

```text
test a_fixed_port_still_prints_that_port ... ok
test port_zero_prints_the_port_the_kernel_granted ... ok
test result: ok. 2 passed; 0 failed
```

O teste abre o binário, lê a primeira linha do `stdout`, exige porta ≠ 0 e faz `GET /health` na porta
impressa, conferindo que o `pid` da resposta é o do filho — a mesma prova que o helper do E2E faz.

## (b) O spec do helper: 6/6

```sh
bunx playwright test tests/e2e/isolated-host.spec.ts
```

```text
✓  1 isolated-host.spec.ts:34:1 › refuses a /health that belongs to another process (116ms)
✓  2 isolated-host.spec.ts:59:1 › accepts the host it started and reports the port the kernel granted (1.2s)
✓  3 isolated-host.spec.ts:71:1 › a second host gets its own port (1.1s)
✓  4 isolated-host.spec.ts:81:1 › stop kills the host and removes its home (16ms)
✓  5 isolated-host.spec.ts:90:1 › prepare seeds the home and its env before the host starts (1.1s)
✓  6 isolated-host.spec.ts:112:1 › a prepare that throws leaves no home behind (4ms)

  6 passed (10.4s)
```

O decoy mudou de forma, não de papel: um `http.createServer` numa porta efêmera responde `/health` com
`pid: 999999`, e o filho de mentira (o próprio `process.execPath` com `-e`) imprime a linha do bridge
apontando para **essa** porta. O helper lê a porta, sonda o `/health`, vê outro `pid` e recusa nomeando
a porta e `999999` — a recusa agora acontece dentro do caminho novo (ler a porta da saída do filho), e
o teste 3 mostra dois hosts do mesmo processo com portas e pids diferentes.

## (c) Duas sessões em paralelo no mesmo worktree: 6/6 + 6/6

```sh
# sessão A: 1520/4417          # sessão B: 1521/4418
bunx playwright test tests/e2e/trust-prompt.spec.ts --output=/tmp/pw-a &
bunx playwright test tests/e2e/welcome.spec.ts --output=/tmp/pw-b &
wait
```

```text
A: 6 passed (1.4m)   trust-prompt.spec.ts
B: 6 passed (41.8s)  welcome.spec.ts
```

Nenhuma das duas escolheu porta auxiliar: cada host próprio subiu em `--port 0` e as duas rodaram ao
mesmo tempo no mesmo worktree, com homes diferentes (derivados das portas da UI).

**Achado da primeira tentativa, registrado porque é armadilha de quem repetir:** sem `--output`
separado, as duas sessões escrevem em `test-results/` do worktree e uma apaga o trace da outra —
`browserContext.close: ENOENT ... test-results/.playwright-artifacts-0/traces/...`. Não é porta nem
fixture: é o diretório de artefatos, que o Playwright limpa no começo de cada run. O `AGENTS.md`
registra o `--output` próprio ao lado da nota de que a fixture é compartilhada.

## (d) Os sete specs de host próprio: 11 passed, 7 skipped

```sh
bunx playwright test tests/e2e/environment.spec.ts tests/e2e/analysis-gate.spec.ts \
  tests/e2e/setup-gate-layout.spec.ts tests/e2e/medium-editor-session.spec.ts \
  tests/e2e/tools-install-ux.spec.ts tests/e2e/debug-advanced.spec.ts
```

```text
✓ analysis-gate.spec.ts (2)          ✓ environment.spec.ts (3)
✓ medium-editor-session.spec.ts (2)  ✓ setup-gate-layout.spec.ts (2)
✓ tools-install-ux.spec.ts (2)       - debug-advanced.spec.ts (7 skipped: sem netcoredbg real)
  11 passed, 7 skipped (1.8m)
```

`debug-advanced` faz `test.skip(!fs.existsSync(realAdapter))` nesta máquina (o adaptador real não está
instalado): o `beforeAll` sobe o host efêmero e os sete testes são pulados. **Não verificado:** as
asserções de hit count/watch do `debug-advanced` — skip não é passe.

## (e) O resto do que o CI roda

```sh
bun test tests/unit           # 127 passed, 0 fail
bunx tsc --noEmit             # limpo
bun run build                 # tsc + vite build, ok
cargo fmt --all -- --check    # limpo
cargo clippy --workspace --all-targets -- -D warnings   # limpo
cargo test -p openridr-host   # 470 + 11 + 2 + 15 + 109 verdes, com as falhas de carga abaixo
```

`cargo test -p openridr-host` no run completo teve 1–2 falhas da classe de **carga** já registrada em
`docs/evidence/test-stability.md` (`lsp_completion_and_definition_…` e
`run_start_reports_restore_required_when_assets_are_invalidated_after_a_build`, que dependem de
`csharp-ls`/`dotnet` reais). Rodados isolados, os dois passaram (`1 passed` em 19,66 s e `1 passed` em
5,11 s), e nenhuma linha de `crates/host` mudou nesta frente. O job `rust` do CI roda
`cargo test -p openridr-host`, que **não** inclui o teste novo de `host-http`: ele roda em
`cargo test --workspace` e localmente.

## O que não foi verificado

- `debug-advanced.spec.ts`: as 7 asserções pulam sem o netcoredbg real (skip, não passe).
- Windows e macOS Intel: fora do alvo.
- Chrome real: esta máquina só tem o Chromium do Playwright.
- O smoke noturno: a lista de specs não mudou nesta frente.
