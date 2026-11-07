# Retro — risco de PR por caminho tocado (issue #74)

Sessão de 2026-11-06/07. Trabalho: [SPEC-RISK-TIERS](SPEC-RISK-TIERS.md), issues #122 a #126 (e a filha
#139), PR #131. Retro sobre **o ambiente de trabalho do agente**, não sobre o produto.

## O que funcionou

- **O gate provou a si mesmo no próprio PR.** O PR é `risk:high` (toca `.github/workflows/**`,
  `AGENTS.md`, `scripts/ci-*`), então o primeiro run real já era a prova: o job `risk` reprovou com
  `high-risk pull request: it needs an approval from the maintainer of the current revision (6ac508a)`,
  o agregador `ci` reprovou com `risk: failure`, e o label `risk:high` apareceu sozinho. Nenhum teste
  sintético substitui isso: o caminho "sem aprovação → vermelho" foi observado de verdade.
- **Autoteste com repositório git de verdade, não com mock.** `scripts/ci-risk-tier-selfcheck.sh` cria
  commits reais em `mktemp` e roda o script contra eles. Isso deixou visíveis três armadilhas que um mock
  esconderia: `[ x ] && y` sob `set -e` aborta o script; o loop sobre caminhos expandia globbing; e um
  fixture meu (`"devDependencies": {}`) era uma mudança de dependência legítima — o **script** estava
  certo e o fixture errado.
- **Falha-antes medida por mutação, não afirmada.** Cada rodada de aperto tem o número: 33 de 43 casos
  falham com um stub `echo low`; 12 de 16 falham com o gate anterior (timestamp); 3 de 43 falham com o
  classificador anterior aos consertos de TOML/merge-base. A segunda mutação é a que importa: ela mostra
  que o aperto foi real.
- **A revisão independente pagou-se.** Três achados altos, todos reais, e o pior deles era estrutural: o
  gate executava código do próprio PR (`actions/checkout` num `pull_request` traz o merge commit), então
  um PR que editasse `scripts/ci-*` se aprovava. Nenhum teste meu pegaria isso — é uma propriedade do
  ambiente do GitHub, não do código. Revisão adversarial em modelo diferente é o único lugar onde essa
  classe aparece.
- **O bootstrap foi tratado como estado, não como exceção escondida.** Quando a base ainda não tem o
  script, o job emite `::warning::` e uma linha "Bootstrap" no resumo — e o run real mostrou as duas
  linhas. Um fallback silencioso teria virado o buraco que a correção fechou.

## O que custou tempo

1. **`shellcheck` faltando na máquina — de novo.** O `RETRO-CI-PR` já tinha proposto escrever
   `brew install shellcheck` no `AGENTS.md`; a proposta não foi aplicada, e o custo voltou: o
   `actionlint` local passou (roda sem shellcheck), o job `governance` do run real reprovou com `SC2006`
   e `SC2016`, e só então instalei o shellcheck. Foram um commit e um run de ~4 min.
   → **Aplicado neste PR** (seção de CI do `AGENTS.md`): `brew install shellcheck dash` e o comando de
   verificação. Era proposta repetida do `RETRO-CI-PR` sem dono; repetir uma terceira vez seria ruído.
2. **`/bin/sh` do macOS não é o do runner.** Escrevi `$((10#$mm + 1))` — conversão de base é indefinida em
   POSIX sh e o `/bin/sh` do Ubuntu é dash. No Mac `sh` é bash, então passou; o shellcheck pegou depois.
   → **Proposta:** `brew install dash` e uma linha no `AGENTS.md`: "os scripts de `scripts/` e
   `tests/shell/` rodam com `dash <arquivo>`, não com `sh`". Melhor ainda: `scripts/shell-tests.sh` que
   roda todos os `tests/shell/*.test.sh` sob `dash`, para o comando ser um só.
3. **Conflito em documento append-only, duas vezes.** `docs/decisions.md`, `GLOSSARY.md` e a tabela do
   `docs/README.md` recebem uma linha por frente; dois `git merge origin/main` deram conflito e cada um
   custou uma resolução manual (manter os dois lados, na ordem em que entraram). Não é difícil, é
   recorrente: aconteceu em toda frente que toca `decisions.md`.
   → **Aplicado neste PR**: `docs/README.md` ganhou a regra de resolução (as duas entradas, a desta
   branch primeiro, nunca reordenar as antigas).
4. **`gh ... --body "$(cat <<'EOF' ... EOF)"` quebra com crase.** O corpo da issue #122 tinha
   `` `scripts/ci-aggregate.sh` `` e o bash tentou substituir comando dentro do heredoc: `bad
   substitution`. Custou uma chamada.
   → **Aplicado neste PR**: `docs/agents/issue-tracker.md` passou a mandar `--body-file` com heredoc em
   arquivo, e explica por que a forma `--body "$(cat <<'EOF' …)"` quebra.
5. **Esperar o CI por `gh run watch`.** Duas tentativas abortaram por timeout do meu lado, e o
   `sleep N && gh pr checks` foi mais barato e mais informativo (mostra o estado de cada job).
   → **Proposta:** nada a mudar no repo; é nota de sessão.
6. **Um flake custou um run inteiro de Rust (3 min 41 s + 8 min de re-execução).** `tools.install` do
   csharp-ls terminou `failed` com log vazio em 1 de 3 rodadas completas. Registrado em
   [evidence/test-stability.md](evidence/test-stability.md) com o sintoma exato; o teste passa isolado.
   → **Proposta em aberto** (não aplicada): o `ensure_tool` do harness deveria incluir o log da instalação
   na mensagem de falha mesmo quando ele vem vazio, e o `tools.install` deveria registrar o motivo de um
   `failed` sem log. Fica fora deste PR porque é mudança em `crates/host/tests/` e este PR é de CI; o
   sintoma está em [evidence/test-stability.md](evidence/test-stability.md) para quem pegar.

## Números

- 12 commits na branch (8 de trabalho, 2 merges de `main`, 2 de correção da revisão), 5 tickets fechados
  (#122–#126) e 1 filha aberta (#139).
- 74 casos de teste de shell (43 do classificador, 16 do gate, 11 do revert rate, 13 do agregador),
  todos verdes também sob `dash`. 4 mutações registradas com o número de casos que cada uma derruba.
- Suíte completa: `bun run test:unit` 86/86, `cargo test -p openridr-host` 480 passando (1 flake em 3
  rodadas), `fmt`/`clippy`/`tsc` limpos.
- Runs reais: 3 no PR #131. O último (`6ac508a`) com TS 16 s, governance 18 s, security 16 s, Rust
  3 min 44 s verdes, `risk` vermelho **de propósito**, `ci` vermelho por consequência.
- Revisão independente: 3 altos, 4 médios, 2 baixos — todos corrigidos, nenhum adiado.
