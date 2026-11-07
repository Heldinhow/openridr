# Evidência — Práticas de CI do T3 Code e do OpenChamber (issue #142)

Spec: [SPEC-CI-PRACTICES.md](../SPEC-CI-PRACTICES.md). Decisões: `docs/decisions.md` (2026-11-07,
"Três buracos medidos do CI fechados sem dependência nova e sem tocar no ruleset"). Tickets: #218
(type check estrito), #219 (job `rust-tauri`), #220 (`dependency-audit.yml`), #221 (inventário,
cobertura e evidência), #238 (`vite build` no job `typescript`).
Ambiente: macOS 27.2, Apple Silicon (arm64), `bun 1.3.9`, `rustc 1.99.0`, `cargo-audit 0.22.2`,
SDK .NET 8.0.408. Base: `40b7a1f` (`ci/issue-142-ci-practices`, head do PR #225).

## Runs de CI no SHA entregue

O SHA entregue é o head da branch no último push de código, `05b44af`. O commit que só acrescenta
este arquivo vem depois dele e tem o seu próprio run, linkado no corpo do PR #225 — a regra do
repositório é a mesma do `ci.yml`: o que prova o código é o run do commit de código, não o do commit
de documentação que o descreve.

| Run | Gatilho | SHA | Resultado | Parede |
|---|---|---|---|---|
| [`42398331889`](https://github.com/Heldinhow/openridr/actions/runs/42398331889) | `pull_request` (PR #225) | **`05b44af`** | **7 jobs verdes** | 5:32 |
| [`42398331892`](https://github.com/Heldinhow/openridr/actions/runs/42398331892) | `pull_request` (filtro de caminho) | **`05b44af`** | `Dependency audit` verde em 31 s | 31 s |
| [`42395785621`](https://github.com/Heldinhow/openridr/actions/runs/42395785621) | `pull_request` (PR #225) | `40b7a1f` | 7 jobs verdes | 4:22 |
| [`42395791925`](https://github.com/Heldinhow/openridr/actions/runs/42395791925) | `workflow_dispatch` | `40b7a1f` | `Dependency audit` verde em 29 s | 29 s |
| [`42395786265`](https://github.com/Heldinhow/openridr/actions/runs/42395786265) | `pull_request` (filtro de caminho) | `40b7a1f` | `Dependency audit` verde em 29 s | 29 s |

Jobs do run do SHA entregue (`42398331889`), com os horários que a API devolve (`startedAt` →
`completedAt`):

| Job | Início → fim | Duração |
|---|---|---|
| TypeScript (tsc + unit) | 17:51:41 → 17:51:58 | 17 s |
| Governance (workflows, PR title, PR body) | 17:51:41 → 17:52:01 | 20 s |
| Risk tier and maintainer approval | 17:51:41 → 17:51:54 | 13 s |
| security (gitleaks) | 17:51:41 → 17:51:56 | 15 s |
| **Tauri (clippy + tests)** — job novo, informativo | 17:51:42 → **17:53:02** | **1:20** |
| Rust (fmt, clippy, host tests) | 17:51:41 → **17:56:24** | **4:43** |
| `ci` (agregador) | 17:56:26 → 17:56:37 | 11 s |

**Parede: 5:32** — run iniciado às 17:51:06, primeiro job às 17:51:41 (35 s de fila/setup do runner)
e o agregador fechando às 17:56:38.

O `completedAt` do `rust-tauri` é **17:53:02** e o do Rust é **17:56:24**: o job novo terminou 3:22
antes do job que fecha a parede, medido nos horários de conclusão, não por subtração de duração. Como
o agregador só começa depois de todos os jobs, a parede é o Rust — o `rust-tauri` cabe inteiro na
folga dele. No run anterior (`40b7a1f`) foram 2:41 contra 4:08 de Rust, com o mesmo desfecho.

A variação do próprio runner é maior que o job novo: nos nove runs verdes de `ci.yml` de 2026-11-07,
a parede foi de 4:19 a 5:32 e o job Rust de 4:01 a 4:49. Dois deles rodaram em outras branches e sem
o job novo — `42396382712` parede 4:43 (Rust 4:26) e `42396447606` parede 4:51 (Rust 4:27) —, e a
medição anterior da spec, mais cedo no dia, tinha dado parede ≈ 4:20 com Rust 3:38–4:21. Ou seja: o
run entregue é o mais lento da série por causa do Rust (4:43) e dos 35 s de fila, não do job novo,
que rodou em 1:20 e terminou 3:22 antes do Rust.

O PR #225 é `risk:high` (mexe em `.github/workflows/**`), calculado pelo job `risk` no run acima.

## #218 — Type check estrito

- `tsconfig.json` ganhou `noUnusedLocals` e `noUnusedParameters`; `noUncheckedIndexedAccess` ficou
  fora (120 achados, próximo degrau).
- Falha-antes (medida na #218, base `6add72e`, com as flags e sem as correções): **17** TS6133 — 14
  de `noUnusedLocals` e 3 de `noUnusedParameters`, **5** deles em specs de E2E. O corpo da issue
  dizia 13 + 4 e 4 em E2E; o número certo é 14 + 3 e 5.
- Depois: `bunx tsc --noEmit` → **0 erros**; `bun test tests/unit` → **95 pass / 0 fail**, 316
  `expect()`, 13 arquivos, 212 ms; `bun run check` limpo.
- O `onChange` de `DebugAdapterPanel` era **fiação morta, não faltando**: a instalação saiu do painel
  em `650245b` e foi para `useExecutionFlow.ts`, então o prop foi removido do componente e dos dois
  call sites (`WorkspaceView.tsx`, `welcome/ToolchainView.tsx`).
- Nenhum passo novo no workflow: o job `typescript` já roda `bunx tsc --noEmit`, e as flags moram no
  arquivo que ele lê.

## #219 — Job `rust-tauri`

- Job novo em `macos-15`, em paralelo ao Rust: toolchain do `rust-toolchain.toml`, `rust-cache` com
  `workspaces: "src-tauri -> target"`, `mkdir -p src-tauri/resources/csharp-ls
  src-tauri/resources/dotnet`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D
  warnings` e `cargo test --manifest-path src-tauri/Cargo.toml`. Entrou no `needs` do agregador `ci`;
  `PR_ONLY` intocado.
- **Falha-antes medida nesta worktree, sem as pastas de resource** (checkout limpo):
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` → exit 1,
  `resource path 'resources/dotnet' doesn't exist` (o `tauri-build` valida os resources do bundle e
  as duas pastas são vendor dirs gitignored).
- Com as duas pastas vazias: clippy **limpo em 5,1 s** (build já quente da tentativa que falhou) e
  `cargo test --manifest-path src-tauri/Cargo.toml` → **6 testes passando, 0 falhando**, 33,6 s de
  parede (limitado por link).
- No runner: **2:41**, verde, e a parede do run não mudou (tabela acima).
- Informativo por `continue-on-error: true`: o `result` do job vira `success`, o agregador `ci`
  continua verde e o ruleset não muda. Critério de promoção escrito no YAML: 5 execuções verdes
  consecutivas, pelo menos uma com cache frio, duração estável abaixo de 5 min e nenhum flake;
  promover é remover a flag num PR próprio. **Estado hoje: 1 de 5** — este é o único run com o job.

## #220 — `dependency-audit.yml`

- `schedule` semanal (segunda, 04:10 UTC) + `workflow_dispatch` + `pull_request` filtrado por
  `bun.lock`, os dois `Cargo.lock` e o próprio arquivo. `permissions: contents: read`, `concurrency`,
  `timeout-minutes: 15`, checkout sem credencial, actions pinadas pelo mesmo SHA do `ci.yml`. Fora do
  `needs` do agregador e do `PR_ONLY`: não pode bloquear merge.
- `bun audit` direto (0,4 s, lê `bun.lock` e o registry, sem `bun install`); `cargo-audit` 0.22.2 do
  tarball musl pré-compilado com **sha256 fixo no arquivo**, chamado como `cargo-audit audit` (não
  `cargo audit`, que passaria pelo shim do rustup e pelo `rust-toolchain.toml`).
- Sem `continue-on-error`, sem `|| true`, sem cache do advisory-db: scanner que não rodou é vermelho.

Medições locais (todas nesta worktree, base `40b7a1f`):

| Comando | Resultado |
|---|---|
| `bun audit` | `No vulnerabilities found`, exit 0, 0,44 s |
| `bun audit` com registry inalcançável (`BUN_CONFIG_REGISTRY=http://127.0.0.1:1`) | exit 1, `ConnectionRefused: audit request failed` |
| `cargo-audit audit --file Cargo.lock` | 0 vulnerabilidades, **1** aviso unmaintained (`rustls-pemfile` 2.2.0, RUSTSEC-2025-0134), exit 0, 2,2 s |
| `cargo-audit audit --file src-tauri/Cargo.lock` | 0 vulnerabilidades, **6** avisos (glib/zvariant e afins, deps de Linux), exit 0, 1,7 s |
| `cargo-audit audit --no-fetch --db /tmp/nonexistent-advisory-db` | exit 1, `error loading advisory database` |

### Probe de advisory real

Projeto descartável **fora do repositório** (`mktemp -d /tmp/openridr-audit-probe.XXXXXX`) com
`package.json` e `lodash@4.17.15`, `bun install` e `bun audit`:

```
lodash  <4.17.21
  (direct dependency)
  high: Command Injection in lodash - GHSA-35jh-r3h4-6jhm
  high: Prototype Pollution in lodash - GHSA-p6mc-m468-83gw
  moderate: Regular Expression Denial of Service (ReDoS) in lodash - GHSA-29mw-wpgm-hmr9
  high: lodash vulnerable to Code Injection via `_.template` imports key names - GHSA-r5fr-rjxr-66jc
  moderate: Lodash has Prototype Pollution Vulnerability in `_.unset`, `_.omit`, and `_.pullAt` - GHSA-xxjr-mmjv-4gpg

5 vulnerabilities (3 high, 2 moderate)
```

`exit 1`. O probe não entrou no repositório.

## #221 — Inventário, cobertura dos gates e comandos locais

### Inventário conferido contra o repositório

Cada linha da tabela de `docs/SPEC-CI-PRACTICES.md` foi conferida com um comando ou um arquivo; o que
não conferiu foi corrigido na spec.

| Prática da referência | Como foi conferido | Resultado |
|---|---|---|
| Separação de lint/build/typecheck/testes (T3 Code) | `ci.yml`: jobs `typescript` (tsc + build + unit) e `rust`; `grep -rn "vite build" .github/workflows/` | jobs separados, com o `vite build` no job `typescript` desde a #238 |
| Verificação de dependência não usada (T3 Code) | `bunx knip --reporter compact` | 1 linha de dependência não usada (2 pacotes `@fontsource-variable/*`), 1 linha de importada sem declarar (3 pacotes), **8** exports e **7** tipos sem uso — os números da spec conferem |
| Verificação de arquivo gerado (T3 Code) | `.gitignore` (`dist/`, `target/`, `src-tauri/resources/{dotnet,csharp-ls}/`) e `git ls-files` filtrando esses caminhos | vazio: nenhum artefato de build versionado |
| Jobs independentes e paralelos (OpenChamber) | `ci.yml`: 6 jobs + agregador com `needs` | confere |
| Actions fixadas por SHA (OpenChamber) | `grep -rn "uses:" .github/workflows/ \| grep -v "@[0-9a-f]\{40\}"` | 23 `uses:`, **todas** com SHA de 40 hex |
| Controles de segurança na automação de review (OpenChamber) | `permissions:` nos 4 workflows; `persist-credentials: false` nos checkouts; `online-audits: false` no zizmor | 4/4 workflows com `permissions`, 10/10 checkouts sem credencial, zizmor offline |
| Review automatizado por LLM (OpenChamber) | `ci.yml`, comentário do job `security` | não existe; recusado (credencial/serviço pago, aprovação por IA) |
| Integridade e smoke de release | `ls scripts/release*` e `grep -n "self" .github/workflows/release.yml` | os quatro scripts existem, mas **nenhum workflow os chama**: o `release.yml` roda o preflight `release-macos.sh --check`. Linha corrigida |
| Lint de TS determinístico (as duas) | `package.json` (sem Biome/ESLint) e `tsconfig.json` | sem linter; adotado só o type check estrito |
| Cobertura de `src-tauri` | job `rust-tauri` no `ci.yml` + run `42395785621` | adotado, rodou em 2:41 |
| Auditoria de dependências | `dependency-audit.yml` + runs `42395791925` e `42395786265` | adotado, 29 s em cada gatilho |
| Robustez dos gates por caminho | os quatro scripts de CI, caso a caso | confere: 45 + 13 + 23 + 27 casos |

### Item 4 da #142: critério → teste → caso

| Critério | Caso medido | Resultado |
|---|---|---|
| Caminho desconhecido | `scripts/ci-risk-tier-selfcheck.sh`: `unknown-tsconfig`, `unknown-html` | `medium` (o default é o tier do meio, nunca o baixo) |
| Falha de classificação | idem: `err-unknown-base` e `err-empty-diff`; `cargo-unparseable-value` | os dois primeiros saem **2** (o job cai para `high`); valor de `Cargo.toml` não parseável classifica `high` direto |
| Arquivos sensíveis | idem: `workflow`, `ruleset`, `agents-md`, `release-script`, `vendor-script`, `ci-script`, `entitlements`, `tauri-conf`, `trust-model`, `lockfile-cargo`, `lockfile-bun` | todos `high` |
| Alteração do próprio gate | `tests/shell/ci-workflow-bootstrap.test.sh`: `blob-regular` utilizável; `blob-empty`, `blob-symlink`, `blob-tree` e `blob-missing` recusados; `gate-*` e `aggregate-*` | 27 casos passando |
| Mudança do head depois da aprovação | `tests/shell/ci-risk-gate.test.sh`: `approval-on-older-commit`, `withdrawn-by-changes-requested`, `withdrawn-by-dismissal`, `approval-without-write-access`, `api-failure` | 23 casos passando |
| Residual #139 | — | declarado: o YAML do PR roda; exige mudança de ruleset, decisão do Helder |

Os quatro scripts, medidos agora: `scripts/ci-risk-tier-selfcheck.sh` **45 casos**,
`tests/shell/ci-aggregate.test.sh` **13**, `tests/shell/ci-risk-gate.test.sh` **23**,
`tests/shell/ci-workflow-bootstrap.test.sh` **27** — todos exit 0. (`tests/shell/` tem mais dois
arquivos, `git-remote-fixture.test.sh` e `revert-rate.test.sh`, que não são gates de risco: 10 e 12
casos, também verdes.)

No run do PR #225 o tier calculado foi `high` e o gate passou pela regra do **autor OWNER**
(2026-11-07): o caminho de aprovação por review de outra pessoa não foi exercitado por um run real,
só pelos casos de `ci-risk-gate.test.sh`.

### Comandos locais

`AGENTS.md`, seção **Comandos**, ganhou os dois comandos do job `rust-tauri` e a nota de que o CI não
roda os vendor scripts (`resources/csharp-ls/` e `resources/dotnet/` são gitignored e o `tauri-build`
só exige que o caminho exista). Na seção de CI, a linha do job `rust-tauri` na tabela, o critério de
promoção e a correção do bullet "`src-tauri` fica fora do clippy", que deixou de ser verdade.

## #238 — `vite build` no job `typescript`

O job `typescript` do `ci.yml` ganhou o passo `Build` (`bunx vite build`) entre "Type check" e
"Unit tests", e passou a se chamar `TypeScript (tsc + build + unit)`. `bunx vite build` e não
`bun run build` porque o `tsc` já rodou no passo anterior. Nenhum cache de `dist/`: o artefato é
descartável no runner.

Medido nesta worktree (macOS arm64, `bun 1.3.9`, base `baa9821`), com `bun install` antes:

| Comando | `src/main.tsx` | Exit | Duração |
|---|---|---|---|
| `bunx vite build` | limpo | **0** | 1,57 s (o Vite reporta `built in 545ms`) |
| `bunx tsc --noEmit` | `import "./does-not-exist.png"` na primeira linha | **0** | 1,26 s |
| `bunx vite build` | idem | **1** (`[UNRESOLVED_IMPORT] Could not resolve './does-not-exist.png' in src/main.tsx`) | 0,39 s |

A terceira linha é a falha-antes: o `tsc` aceita o import porque `vite/client` declara `*.png`, e só
o bundler recusa. O import quebrado foi revertido antes do commit e `src/main.tsx` ficou sem diff.

## O que é configuração existente, o que foi executado e o que não foi

- **Configuração existente, não exercitada por esta frente**: os gates de risco e o ruleset
  (`.github/rulesets/main.json` não é aplicado pela sessão); o `schedule` semanal do
  `dependency-audit` e o smoke noturno.
- **Execução comprovada**: os cinco runs da tabela do topo — no SHA entregue `05b44af`, os 7 jobs do
  `ci` e o `dependency-audit`; em `40b7a1f`, os mesmos 7 jobs e o `dependency-audit` por
  `workflow_dispatch` e por `pull_request`; `bunx tsc --noEmit` (0), `bunx vite build` (0, 1,57 s),
  `bun test tests/unit` (95/0), `bun run check`, clippy e os 6 testes de `src-tauri`, `bun audit`,
  `cargo-audit` nos dois lockfiles, o probe com advisory real, as duas falhas de rede dos scanners e
  os quatro scripts de CI (45 + 13 + 23 + 27 casos). `knip` foi remedido para conferir o inventário.
- **Não verificado**: o `schedule` semanal (só se prova depois do merge na default); o critério de
  promoção do `rust-tauri` (**2 de 5** runs verdes); o caminho de aprovação por review de um PR
  `risk:high` de autor que não é o dono (coberto por teste, não por run); o caminho "um scanner falha
  e os outros dois ainda rodam" — o `if: always()` está no YAML e foi lintado, mas não foi exercitado
  com uma falha real.
- **Nada de UI**: nenhuma tela foi tocada, nenhum Playwright, nenhum app Tauri aberto. A superfície
  verificável aqui é YAML e shell, e ela foi exercitada de verdade.
