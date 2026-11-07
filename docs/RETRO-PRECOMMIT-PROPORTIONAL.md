# RETRO — pre-commit proporcional ao diff (issue #265, ticket #271)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-PRECOMMIT-PROPORTIONAL](SPEC-PRECOMMIT-PROPORTIONAL.md).
Evidência: [evidence/precommit-proportional.md](evidence/precommit-proportional.md). Origem: o candidato
nº 1 do [RETRO-CI-PRACTICES](RETRO-CI-PRACTICES.md), onde ~16 s de `cargo check` por linha de Markdown
empurraram uma sessão para `--no-verify` num amend.

## O que se sustentou

- **O fato decisivo foi medido antes da primeira pergunta.** Num repositório descartável, `git commit
  --only` aponta `GIT_INDEX_FILE` para um index temporário — logo `git diff --cached --name-only` dentro
  do hook é exatamente o commit, e `git diff --name-only HEAD` over-reporta a árvore suja. Isso virou a
  primeira linha da spec e nunca precisou de decisão do orquestrador; as sete perguntas do grill foram
  só de escopo e política.
- **O seam já existia e não custou nada.** O hook é a fronteira de processo: o teste faz `git commit` de
  verdade com `bun`/`cargo` falsos no `PATH`. Nenhuma variável só-de-teste no hook, nenhum refactor para
  testar, 12 casos em 283 linhas sem compilar nada.
- **O implementer subagent entregou no primeiro round**, com a falha-antes medida por ele (hook antigo →
  18 `FAIL`) e sem tocar em `bun run check`, `commit-only.sh` ou no CI.
- **O code review pagou o preço.** Achou dois caminhos em que o hook ficava **mais fraco** que o antigo:
  nome não-ASCII citado pelo git (`"crates/a\303\247..."`) não casava com `crates/*` e pulava o
  `cargo check`; renomeação listava só o destino, então mover um `.rs` para `docs/` passava como
  documentação. Os dois viraram caso de teste e conserto (`core.quotePath=false`, `--no-renames`, e um
  ramo que roda os dois checks quando o git insiste em citar o nome).
- **A medição A/B saiu barata** com `CARGO_TARGET_DIR` apontando para o `target/` do worktree: 11,6 s
  (hook antigo) contra 108 ms (hook novo) num commit só de Markdown, sem esperar cargo frio.

## O que quebrou / custou

- **Meu script de medição morreu no primeiro caso de falha esperada.** `set -e` com `git commit; e=$?`
  aborta no commit que deve falhar (sem `node_modules`), e a medição teve que ser refeita. A armadilha é
  a mesma de sempre: `set -e` e `cmd; status=$?` não combinam.
- **A primeira medição do commit "misto" não era mista.** O `.rs` já estava commitado, então o
  `git commit --only` levava só o `.tsx` e o hook rodou só `tsc` — 618 ms em vez de ~1,4 s. Medir sem
  conferir o que o commit levou produz número enganoso; a segunda tentativa, com os dois arquivos sujos,
  é a que está na evidência.
- **A mensagem "só documentação, nada a checar" mentia no amend de mensagem** (diff vazio, documento
  nenhum mudou). Virou `pre-commit: nada a checar`, com o caso do amend pinando a linha.
- **`git diff --name-only` não é "a lista de caminhos".** É a lista *citada* (não-ASCII, tab, aspas) e
  *deduplicada por renomeação*. O nome da opção esconde as duas transformações, e as duas fazem o hook
  pular um check em silêncio — o pior modo de falha para um guardrail.
- **O teste novo não roda no CI.** `tests/shell/` continua sem job; a prova é "rodei à mão, sob `sh` e
  sob `dash`". A #269 já existe para isso.

## Candidatos para o ambiente (por severidade)

1. **`scripts/ci-risk-tier.sh:40` tem o mesmo bug de citação.** Ele já usa `--no-renames`, mas não
   `core.quotePath=false`: um PR que toca só `docs/evidência.md` chega citado, não casa com `docs/*` e
   cai no tier default (`medium`) em vez de `low` — o auto-merge não arma e o mantenedor é chamado à toa.
   Uma flag (`git -c core.quotePath=false diff …`) fecha a classe; é `scripts/ci-*`, então issue própria
   e PR de risco alto.
2. **Ligar `tests/shell/*.test.sh` no CI** (issue #269, já aberta). Este teste é o nono da pasta e
   continua sem guardrail: um `dash` que quebre o hook passa despercebido até alguém commitar.
3. **`src-tauri` fora do `cargo check` do hook.** Um commit que mexe só em `src-tauri/**` paga um
   `cargo check --workspace --all-targets` que não cobre o que mudou. Cobrir exige
   `--manifest-path src-tauri/Cargo.toml` (compila o Tauri inteiro), então provavelmente é trabalho do
   job `rust-tauri`, não do hook — decisão de custo, não de código.

## Lições

- **Um guardrail que decide por caminho precisa desconfiar do formato em que o git entrega os caminhos.**
  Citação e renomeação são transformações silenciosas; o default tem que ser "checar mais", nunca
  "checar menos".
- **O aviso de escopo do briefing vira achado de review.** "O hook não pode ficar mais fraco" foi o que
  fez o reviewer procurar exatamente os caminhos em que a classificação erra — e achou dois.
- **Medir é conferir o que foi medido.** O caso "misto" que não era misto e o `set -e` que matou o script
  custaram uma rodada cada; ambos eram visíveis no log antes de virar número na evidência.

## Não verificado

- Superfícies A/B/C do `verify-openridr`: nenhuma interface, host ou app mudou — é o hook de git.
- `bun run check` completo rodou uma vez (7,9 s, verde); `bun test tests/unit` 123 pass. O hook em si
  nunca rodou `cargo check` de `src-tauri` (limite declarado).
- Windows e macOS Intel, como no resto do repositório.
