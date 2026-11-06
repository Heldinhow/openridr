# Evidência — CI em pull requests (issue #56)

Spec: [SPEC-CI-PR.md](../SPEC-CI-PR.md). Decisões: `docs/decisions.md` (2026-11-06, "CI em PR").
Tickets: #80 (fmt), #81 (clippy), #82 (workflow), #83 (governança), #84 (segredos), #85 (Dependabot),
#86 (smoke noturno), #87 (ruleset preparado), #88 (AGENTS.md).
Ambiente: macOS 27.2, Apple Silicon (arm64), `bun 1.3.9`, `rustc 1.99.0`, `SDK .NET 8.0.408`,
`actionlint 1.7.12` (compilado de `github.com/rhysd/actionlint/cmd/actionlint@v1.7.12`),
`zizmor 1.32.0` (`uvx --from zizmor==1.32.0`). Base: `4df5c1a` (`origin/main`).

## PR mecânico (`ci/issue-56-fmt`, #80 e #81)

| Antes | Depois | Comando |
|---|---|---|
| 596 hunks de `fmt` em 33 arquivos (32 no workspace + 1 em `src-tauri`) | `cargo fmt --all -- --check` e `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` limpos | `cargo fmt` |
| 51 linhas de `warning` do clippy (~40 avisos distintos, 22 tipos de lint) | `cargo clippy --workspace --all-targets -- -D warnings` limpo | idem |

- O commit de formatação (`style: format the repository with cargo fmt`) tem 2808 inserções e 1277
  remoções, todas saídas do `cargo fmt`: nenhuma edição manual. O hash completo está em
  `.git-blame-ignore-revs`; `git config blame.ignoreRevsFile` aplicado e conferido com
  `git blame -L 1,1 crates/host/src/build.rs` (linha atribuída ao commit original de 2026-10-28, não
  ao de formatação).
- Clippy: `--fix` resolveu o mecânico (22 dos 40 avisos, 15 arquivos) e o resto foi a mão. Revisão do
  diff de `--fix`: `map(...).flatten()` → `and_then`, `if let/else return` → `?`, `chunks_exact(2)` →
  `as_chunks::<2>()`, `map_or(true, …)` → `is_none_or`, `sort_by(cmp)` → `sort_by_key`,
  `io::Error::new(Other, …)` → `io::Error::other`, `map.values()` e `slice::from_ref` em testes — todas
  equivalências diretas, nenhuma mudança de comportamento. Os quatro `too_many_arguments` e o
  `large_enum_variant` de `build::Line` ficaram `#[allow(...)]` com comentário de uma linha; `Default`
  para `Tools` e `Registry` virou `impl` real.
- Suíte depois do clippy: `cargo test -p openridr-host` → 7 binários, **427 testes passando, 0
  falhando** (`lib` 286, `host` 98, `debugger` 14, `bundled_analysis` 11, `test_discovery` 10,
  `test_run` 7, `trust_model` 1), 3 min 59 s de parede com build quente.

## PR de CI (`ci/issue-56-pr-checks`)

### Gate do corpo do PR (#83)

- `sh scripts/ci-pr-body-selfcheck.sh` → **13 casos, 13 ok**: corpo completo (aceito), heading em
  português (aceito), `Refs #N` sem fechar (aceito), sem issue (reprovado), issue dentro de bloco de
  código (reprovado), comentário HTML de várias linhas como Evidência (reprovado), comando dentro de
  bloco de código como Evidência (aceito), com issue e sem Evidência (reprovado), só `n/a`
  (reprovado), só o comentário-placeholder do template (reprovado), `dependabot[bot]` e
  `renovate[bot]` (isentos) e o **template real** preenchido só com o número da issue (reprovado).
- Mesmo resultado com `LC_ALL=C` (o heading acentuado não depende de locale).
- O self-check não é vazio: com um gate falso (`exit 0`) no lugar do script, ele falha no terceiro
  caso (`no-issue: expected rejected, got accepted`). Com o gate anterior à revisão, falha em
  `issue-inside-code-fence: expected rejected, got accepted` — as falhas-antes de cada rodada de
  aperto do gate ficam registradas aqui.
- O caso `template-with-issue` lê `.github/pull_request_template.md` de verdade: se alguém transformar
  de novo uma instrução em texto solto, o caso começa a aceitar e reprova.
- Agregador (`ci`): lógica testada com `jq` sobre os quatro resultados possíveis — PR tudo verde
  (passa), `push` com `governance` `skipped` (passa), PR com `rust` falhando (reprova, `rust:
  failure`), `push` com `rust` `skipped` por acidente (reprova, `rust: skipped`). A versão anterior
  aceitava qualquer `skipped`.

### Workflows (#82, #83, #84, #86)

- `actionlint 1.7.12`: `actionlint -oneline` sobre `.github/workflows/` → exit 0, sem achado.
  `shellcheck` não está instalado nesta máquina, então a análise de shell do `run:` **não rodou aqui**
  (o job do CI liga shellcheck por padrão; ver "Não verificado").
- `zizmor 1.32.0`, antes das correções no `release.yml`: `error[cache-poisoning]` (cache de bun em
  workflow que publica), `warning[artipacked]` (checkout sem `persist-credentials: false`) e 8
  `info[template-injection]`; exit 14.
- Depois de `persist-credentials: false` no checkout e `no-cache: true` no `setup-bun` do
  `release.yml`: só os 8 `info[template-injection]` (todos expansão de `steps.*.outputs.*` do nosso
  `release-macos.sh` dentro de `run:`). Com `--min-severity low` (o que o job usa): **0 achados, exit
  0**; com `--min-severity medium`: também 0.
- Lista do smoke E2E, lida com o mesmo `sed`/`while read` do workflow: `shell workspace trust-prompt`
  → `running: tests/e2e/shell.spec.ts tests/e2e/workspace.spec.ts tests/e2e/trust-prompt.spec.ts`
  (exit 0); lista só com comentários → `list is empty`, exit 1; nome sem `.spec.ts` → exit 1.
- `.github/rulesets/main.json`: `json.load` ok, regras `deletion`, `non_fast_forward`,
  `required_linear_history`, `pull_request`, `required_status_checks` com contextos `ci` e `security`.

### TypeScript (não tocado, medido para o CI)

`bunx tsc --noEmit` → 0 erros; `bun test tests/unit` → 39 testes, 151 asserts, 497 ms.

## O que não foi verificado

- **Nenhum workflow rodou no GitHub.** Não houve push nesta sessão até o fim dela: os nomes `ci` e
  `security`, o `aggregate` e o `workflow_dispatch` existem só como YAML validado por `actionlint` e
  `zizmor`. O primeiro run real é o do PR — e é ele que decide se o `rustup toolchain install` acha a
  1.99.0 na imagem `macos-15`, se o `setup-dotnet` baixa 8.0.408 e se os testes do host passam num
  runner limpo.
- **`shellcheck` sobre os `run:`**: indisponível localmente. É o único check do `actionlint` que não
  rodou.
- **`min-severity: low` no zizmor** é o que o job usa; o efeito no *exit code* foi medido pela CLI
  (`zizmor --min-severity low` → 0), não pela action.
- **Ruleset não aplicado** e não validado pela API: o JSON é parseável e segue a forma documentada,
  mas o `POST` não foi feito (decisão do Helder) e não há dry-run na API para conferir antes.
- **`src-tauri` fica fora do clippy** no CI (compila o Tauri inteiro); o `fmt` dele entra.
- **Release**: as duas linhas mexidas no `release.yml` (`persist-credentials: false`, `no-cache: true`)
  não foram exercitadas — o próximo release por tag é a primeira execução real.
