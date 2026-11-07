# Host auxiliar do E2E com identidade provada (issue #216)

Medição em macOS Apple Silicon (arm64), macOS 27.2, 2026-11-07, Chrome real (`channel: "chrome"`),
Playwright 1.64.1, bun 1.3.9, node 26.10.0, SDK 8.0.408 das fixtures em `~/.dotnet`, `workers: 1`.
Par de portas da sessão: **1620 (UI) / 4517 (host)**; auxiliares 4518–4524, conferidas livres com
`lsof -nP -iTCP:<porta> -sTCP:LISTEN` antes de começar. Branch `fix/issue-216-e2e-host-ownership`,
HEAD `0b8d81b` (#233 e #234 mergeados). Logs e capturas fora do repositório (`/tmp`).

## O que foi medido

1. O spec do helper (`isolated-host.spec.ts`) verde, incluindo o **decoy** que responde `/health` com
   pid alheio: o helper recusa e a mensagem nomeia a porta e o pid estranho.
2. Os **sete specs** migrados verdes no mesmo par de portas, sem skip.
3. O **"antes"**: o poll que os sete usavam (fetch de `/health` até responder `ok`, sem olhar o pid)
   aceita um decoy que responde com pid estranho — reproduzido num script descartável.
4. A premissa do `exec`: com `cargo run`, o pid do filho **é** o pid que o `/health` devolve, e
   `kill()` no filho derruba o host.

## (a) O spec do helper: 4/4

```sh
OPENRIDR_E2E_UI_PORT=1620 OPENRIDR_E2E_HOST_PORT=4517 \
bunx playwright test tests/e2e/isolated-host.spec.ts
```

```text
✓  10 isolated-host.spec.ts:25:1 › refuses a port whose /health belongs to another process (69ms)
✓  11 isolated-host.spec.ts:47:1 › accepts the host it started and reports its pid and home (718ms)
✓  12 isolated-host.spec.ts:56:1 › stop kills the host and removes its home (2ms)
✓  13 isolated-host.spec.ts:65:1 › prepare seeds the home and its env before the host starts (415ms)
```

O primeiro teste é o decoy: um `http.createServer` na porta auxiliar responde `/health` com
`pid: 999999`; `startIsolatedHost` rejeita, a mensagem contém a porta (`4518`) e `999999`, e o
`mkdtemp` da tentativa falha não deixa diretório em `os.tmpdir()`. O segundo compara o pid do
`/health` com o pid do filho (`expect(health.pid).toBe(host.pid)`), que é a prova de posse; o terceiro
prova que `stop()` mata o host e apaga o home.

## (b) Os sete specs: 31/31, 3,2 min

```sh
PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_UI_PORT=1620 OPENRIDR_E2E_HOST_PORT=4517 \
OPENRIDR_SCREENSHOT_DIR=/tmp/235-run/shots \
bunx playwright test tests/e2e/isolated-host.spec.ts tests/e2e/trust-prompt.spec.ts \
  tests/e2e/welcome.spec.ts tests/e2e/medium-editor-session.spec.ts \
  tests/e2e/debug-advanced.spec.ts tests/e2e/tools-install-ux.spec.ts \
  tests/e2e/analysis-gate.spec.ts tests/e2e/setup-gate-layout.spec.ts
```

```text
Running 31 tests using 1 worker
  31 passed (3.2m)
```

| Spec | Testes | Porta auxiliar | Resultado |
|---|---|---|---|
| `isolated-host.spec.ts` | 4 | `hostPort + 1` (4518) | 4 passed |
| `trust-prompt.spec.ts` | 6 | `hostPort + 1` (4518) | 6 passed |
| `welcome.spec.ts` | 6 | `hostPort + 2` (4519) | 6 passed |
| `medium-editor-session.spec.ts` | 2 | `hostPort + 3` (4520) | 2 passed |
| `debug-advanced.spec.ts` | 7 | `hostPort + 4` (4521) | 7 passed |
| `tools-install-ux.spec.ts` | 2 | `hostPort + 5` (4522) | 2 passed |
| `analysis-gate.spec.ts` | 2 | `hostPort + 6` (4523) | 2 passed |
| `setup-gate-layout.spec.ts` | 2 | `hostPort + 7` (4524) | 2 passed |

Nenhum skip: `debug-advanced` rodou os 7 com o netcoredbg real instalado em
`~/Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092` (o `test.skip` do arquivo não
disparou). A compilação do host nesta worktree (feita pelo `webServer` do harness e reaproveitada pelo
`cargo run` dos specs) levou 17,4 s; o `START_TIMEOUT_MS` de 300 s do helper não foi exercitado.

## (c) O "antes": o poll antigo aceitava o decoy

Script descartável em `/tmp/235-before-poll.mjs` (**não commitado**), rodado com
`PATH=$HOME/.bun/bin:$PATH bun /tmp/235-before-poll.mjs`. Ele faz o que os sete `beforeAll` faziam
antes da #233: sobe um decoy que responde `/health` com `pid: 999999`, `spawn` de
`cargo run -p openridr-host-http -- --port 4599` com `stdio: "ignore"`, e o poll **idêntico** ao dos
specs (só o timeout encurtou de 300 s para 30 s; o decoy responde na hora):

```js
await expect
  .poll(async () => fetch(`http://127.0.0.1:${port}/health`).then((r) => r.ok, () => false), {
    timeout: 30_000,
  })
  .toBe(true);
```

Saída:

```text
decoy listening on 127.0.0.1:4599, answers /health with pid 999999
spawned cargo run -p openridr-host-http -- --port 4599, child pid 34332
old poll: PASSED after 10 ms — it accepted the decoy
/health answered pid 999999; the child the spec spawned was pid 34332
child exit code: 1 — the host died, the spec never read why
same spawn with stderr captured (exit 1):
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
       Running `target/debug/openridr-host-http --port 4599`
  openridr-host-http: cannot bind 127.0.0.1:4599: Address already in use (os error 48)
```

O poll antigo **passou em 10 ms** contra um processo que não era o filho; o filho (pid 34332) morreu
com `Address already in use` e o spec nunca leu o erro — exatamente o "verde provando o estado de
outro `OPENRIDR_HOME`" que a spec descreve. O mesmo cenário no helper novo é o teste do decoy em (a):
rejeita em 69 ms nomeando a porta e o pid.

## A premissa do `exec`, medida direto

Script descartável em `/tmp/235-exec-check.mjs` (**não commitado**), sem o helper: `spawn` de
`cargo run -p openridr-host-http -- --port 4524`, comparação do pid do filho com o pid do `/health`,
`kill()` no filho e nova consulta:

```text
spawned child pid 34378; /health answered pid 34378; same process: true
after kill() on the child (exit null): /health refused
```

O pid do filho é o do host (o `cargo run` faz `exec`) e `kill` no filho derruba o host — a nota de
"host órfão" de `implementation-notes.md` estava errada e foi corrigida nesta frente. Nada sobrou
escutando em 4524 nem em 4599 depois das medições.

## Não verificado

- **Playwright em PR e no smoke noturno.** `isolated-host.spec.ts` não está em
  `scripts/e2e-smoke.txt` (a lista é curada pela #67; acrescentar spec lá é outra decisão). O único
  E2E automatizado continua sendo o smoke noturno, e o CI não roda Playwright.
- **O resto da suíte E2E.** Só os oito specs acima rodaram; os quarentenados (#43/#67) e os demais não
  foram executados nesta frente.
- **Duas sessões reais disputando a mesma porta.** A recusa foi provada com um decoy determinístico,
  não com dois checkouts rodando ao mesmo tempo.
- **Porta efêmera (`--port 0`).** Não implementada: fica como follow-up da retro, sem issue agora
  (motivo na decisão de 2026-11-07 e em `docs/SPEC-E2E-HOST-OWNERSHIP.md`).
- **Windows e macOS Intel.** Fora do alvo verificado; o `exec` do `cargo run` foi medido só no macOS
  arm64.

## Verificação da sessão (branch de integração, depois do merge de `origin/main`)

Rodada independente da sessão, no mesmo par **1620 (UI) / 4517 (host)**, depois de os três tickets
estarem mesclados (`43bcb74`). O slot fixo 1420/4317 **não** foi usado: nada nesta frente depende dele,
e a convenção de portas da #210 permite o par alternativo.

```sh
PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_UI_PORT=1620 OPENRIDR_E2E_HOST_PORT=4517 \
OPENRIDR_SCREENSHOT_DIR=/tmp/216-verify/shots \
bunx playwright test tests/e2e/isolated-host.spec.ts tests/e2e/trust-prompt.spec.ts \
  tests/e2e/welcome.spec.ts tests/e2e/medium-editor-session.spec.ts \
  tests/e2e/debug-advanced.spec.ts tests/e2e/tools-install-ux.spec.ts \
  tests/e2e/analysis-gate.spec.ts tests/e2e/setup-gate-layout.spec.ts
```

```text
Running 31 tests using 1 worker
  31 passed (3.0m)
```

`bun run check` verde (tsc + `cargo check --workspace --all-targets`).

### O critério de aceite, num spec real

Com um decoy respondendo `/health` (`pid: 999999`) na porta auxiliar que `trust-prompt.spec.ts` usa
(4518), o spec **falha no `beforeAll`** em vez de dirigir o decoy:

```text
Error: isolated host on port 4518 failed: port answered /health with pid 999999, expected child pid 40426
    at isolated-host.ts:94
    at tests/e2e/trust-prompt.spec.ts:18:10
```

É o mesmo cenário que, antes da #233, deixava o spec seguir contra o processo alheio (medido em (c)).
Depois da rodada, `lsof` em 1620/4517/4518 voltou vazio.

### Não verificado nesta rodada

- **Portas padrão 1420/4317.** A verificação rodou em 1620/4517; o slot fixo não foi pedido porque nada
  aqui depende dele. O caminho do smoke noturno (default) continua sem medição desta frente.
- Windows e macOS Intel.

## Code review (openai-codex/gpt-6.1-terra, 2026-11-07): achados e destino

1. **Médio — `prepare` lançando deixava o home temporário.** Procede: o `mkdtemp` vinha antes do
   `spawn` e o `fail` só existia depois dele. **Corrigido**: o `fail` agora é definido antes, trata o
   filho ausente e apaga o home também nesse caminho; caso novo em `isolated-host.spec.ts`
   (`a prepare that throws leaves no home behind`).
2. **Médio — o timer de `SIGKILL` continuava armado depois da saída do filho.** Procede como higiene.
   **Corrigido**: `clearTimeout` no evento `exit`. O risco citado (sinalizar um pid reciclado) não se
   aplica — medido: depois da saída o `ChildProcess` do Node não tem mais handle e `kill()` devolve
   `false` sem sinalizar nada.
3. **Médio — o timeout do hook do Playwright cortava o helper antes dos 300 s.** Procede, e era o
   achado mais caro: `trust-prompt` e `welcome` rodam com `timeout: 30_000`, então um build frio de
   `cargo` matava o `beforeAll` sem a mensagem da porta e sem ninguém para parar o host (o `host` ainda
   não existia no `afterAll`). **Corrigido e medido**: `startIsolatedHost` chama
   `test.setTimeout(START_TIMEOUT_MS + 30_000)`. Prova: spec descartável com
   `test.describe.configure({ timeout: 2_000 })` cujo `beforeAll` chamou o helper e dormiu 5 s —
   **passou** (o orçamento do hook subiu); sem a chamada, o mesmo sleep falha com "hook timeout".
   `test.info().timeout` não serve para conferir: ele continua reportando o timeout do teste (medido).
4. **Médio — porta ocupada por um serviço que não responde `/health` válido.** **Não muda**, por
   decisão: nesse caso o filho não consegue bindar e sai, e o helper cai no ramo "child exited with
   code N before answering /health", cuja mensagem já nomeia a porta. O decoy do spec cobre o caso
   oposto (alguém responde `/health`). Um listener mudo não consegue produzir o bug da #216 — verde
   contra host alheio — porque o spec nunca chega a falar com ele.
5. **Baixo — `spawn` que falha esperava 10 s.** Procede: sem pid não há `exit` para aguardar.
   **Corrigido**: `stopProcess` volta na hora quando o filho nunca teve pid.

Depois das correções: `isolated-host.spec.ts` **5 passed** e os sete specs **32 passed** no total, em
1620/4517 (mesma rodada verde do critério de aceite acima).
