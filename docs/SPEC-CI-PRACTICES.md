# SPEC — Práticas de CI aproveitadas do T3 Code e do OpenChamber, sem duplicar os gates existentes

Status: ready-for-agent, issue-mãe #142. Tickets: #218 (type check estrito), #219 (job `rust-tauri`),
#220 (`dependency-audit.yml`), #221 (inventário, cobertura e evidência). Follow-ups fora desta frente:
#222 (linter de TS) e #223 (dependências importadas sem declarar). Insumo:
[research/ci-quality.md](research/ci-quality.md) (pesquisa da #73) e a pesquisa somente leitura de
06/11/2026 citada na #142. Medições de 2026-11-07 nesta worktree, macOS arm64, `bun 1.3.9`,
`rustc 1.99.0`, base `39c4c17`.

## Problem Statement

O CI de PR do OpenRidr já cobre `cargo fmt`, `cargo clippy -D warnings`, os testes do pacote host,
`tsc --noEmit`, os testes unitários de TS, `actionlint`, `zizmor`, gitleaks, o título e o corpo do PR,
e o risk tier. Ainda assim sobram três buracos medidos, e nenhum deles é resolvido por mais
infraestrutura:

- **O type check aceita código morto.** `tsc --noEmit` passa com 0 erros hoje, mas com
  `noUnusedLocals` e `noUnusedParameters` ligados ele acusa **17** símbolos declarados e nunca lidos
  (14 variáveis e 3 parâmetros, 5 deles em specs de E2E). Um deles é um prop `onChange` recebido e
  ignorado — o tipo de defeito que um agente produz em série e que nenhum check atual vê.
- **`src-tauri` está fora do clippy e dos testes.** O job Rust roda `cargo fmt --manifest-path
  src-tauri/Cargo.toml` e mais nada: `src-tauri` é workspace separado, então `--workspace` não o
  alcança. São 6 testes que nunca rodaram em runner nenhum.
- **Nada audita dependência contra vulnerabilidade conhecida.** gitleaks procura segredo, Dependabot
  propõe bump; nenhum dos dois diz "este `Cargo.lock` tem advisory aberto". Medido hoje: 0
  vulnerabilidades nos dois lockfiles, ou seja, o buraco existe mas ainda não morde.

Ao mesmo tempo, a #142 pede que se saiba o que **não** foi adotado e por quê: sem esse inventário, a
próxima sessão reabre a mesma pesquisa e propõe de novo o que já foi recusado.

## Solution

Quatro entregas pequenas, cada uma com o número que a justifica, e nenhuma delas mexe em ruleset,
required check ou settings do repositório:

1. **Type check estrito, sem dependência nova.** `noUnusedLocals` e `noUnusedParameters` entram no
   `tsconfig.json` e os 17 achados são corrigidos. O `tsc --noEmit` que já roda no CI passa a
   pegá-los: nenhum passo novo no workflow, nenhum lugar novo onde o check pode divergir.
2. **Job `rust-tauri`**, informativo, que roda clippy e testes de `src-tauri` em `macos-15`.
3. **Workflow `dependency-audit.yml`**, semanal, com `bun audit` e `cargo audit` nos dois lockfiles —
   falha de rede é vermelho, nunca "limpo".
4. **Documento de inventário e cobertura**: o que cada referência oferece, o que já existe, o que foi
   adotado, o que foi recusado e por quê — mais a tabela de cobertura dos gates de risco que a #142
   pede no item 4.

O que **não** entra: linter de TS (Biome fica numa issue filha com o baseline já medido), `knip`,
filtro de job por caminho, mudança de ruleset, e qualquer coisa em `release.yml`/`platforms.yml` (a
#54 está mexendo neles).

## User Stories

1. Como mantenedor, quero que o type check recuse variável e parâmetro declarados e nunca lidos, para
   que código morto de sessão de agente não chegue na `main`.
2. Como mantenedor, quero que essa recusa venha do `tsc` que já roda, para não ter um segundo check
   para manter e um lugar novo onde ele pode divergir do editor.
3. Como sessão de agente, quero que o editor e o CI concordem sobre "símbolo não usado", para
   descobrir o problema enquanto escrevo e não no PR.
4. Como mantenedor, quero que `src-tauri` passe por `clippy -D warnings`, para que o código que roda
   com o token do app não seja o único sem lint.
5. Como mantenedor, quero que os 6 testes de `src-tauri` rodem no CI, para que eles deixem de ser
   testes que só existem no Mac de quem escreveu.
6. Como mantenedor, quero esse job em paralelo ao job Rust, para não pagar 3 minutos a mais em todo
   PR por causa de um pacote que quase nunca muda.
7. Como mantenedor, quero esse job **informativo** no começo, com critério escrito de promoção, para
   que a primeira execução a frio não bloqueie um PR antes de eu saber quanto ele custa.
8. Como mantenedor, quero saber exatamente o que falta para o job virar bloqueante, para que a
   promoção não seja uma decisão de memória.
9. Como mantenedor, quero que uma vulnerabilidade conhecida em `bun.lock` ou `Cargo.lock` apareça
   sozinha toda semana, para não depender de eu lembrar de rodar auditoria.
10. Como mantenedor, quero que falha de rede no scanner apareça como vermelho, para que "não rodou"
    nunca seja lido como "não tem vulnerabilidade".
11. Como mantenedor, quero que a auditoria **não** bloqueie merge de PR que não mexe em dependência,
    para que um advisory publicado hoje não pare o trabalho de amanhã.
12. Como mantenedor, quero que um PR que mexe em lockfile rode a auditoria na hora, para que a
    dependência nova seja olhada no PR que a introduz, e não uma semana depois.
13. Como mantenedor, quero uma exceção registrada com motivo e data de revisão, para que ignorar um
    advisory seja uma decisão datada e não um `--ignore` esquecido no YAML.
14. Como mantenedor, quero um inventário curto de "o que o T3 Code e o OpenChamber fazem, o que já
    temos, o que adotamos e o que recusamos", para que a próxima sessão não repita a pesquisa.
15. Como revisor, quero a tabela de cobertura dos gates de risco (critério → teste → caso), para
    saber o que já está provado e o que depende da #139.
16. Como sessão de agente, quero os comandos locais equivalentes aos do CI escritos no `AGENTS.md`,
    para poder reproduzir a falha antes de empurrar a branch.
17. Como mantenedor, quero a duração de cada job medida e registrada, para não aceitar nem prometer
    ganho de velocidade sem comparação.

## Implementation Decisions

### Inventário: o que veio de cada referência

| Prática da referência | Já existe no OpenRidr? | Decisão |
|---|---|---|
| Separação de lint/build/typecheck/testes (T3 Code) | Parcial: jobs `typescript` (tsc + unit) e `rust` (fmt, clippy, testes) separados; **nenhum job roda `vite build`** | **Nada a fazer nesta frente**: o tipo é coberto pelo `tsc --noEmit` que já roda; o build da UI nunca rodou no CI — residual declarado, não fechado aqui |
| Verificação de dependência não usada (T3 Code) | Não | **Adiado**: `knip` medido (2 famílias de dependência não usadas, 3 importadas sem declarar, 8 exports e 7 tipos sem uso; remedido em 2026-11-07 com os mesmos números); adotar é dependência nova. As 3 sem declarar viram issue filha (#223). |
| Verificação de arquivo gerado (T3 Code) | Parcial: `dist/` e `target/` são gitignored; `src-tauri/resources/{dotnet,csharp-ls}` também | **Nada a fazer**: não há arquivo gerado versionado para conferir |
| Jobs independentes e paralelos (OpenChamber) | Sim | **Nada a fazer** |
| Actions fixadas por SHA (OpenChamber) | Sim, todas | **Nada a fazer** |
| Controles de segurança na automação de review (OpenChamber) | Parcial: `permissions` mínimas, `persist-credentials: false`, zizmor offline | **Nada a fazer** |
| Review automatizado por LLM (OpenChamber) | Não | **Recusado**: exige serviço pago ou chave de API e aprovação automática por IA. Decisão do Helder, não de sessão. |
| Integridade e smoke de release | Sim: `release-selfcheck.sh`, `release-macos.sh`, `updater-pubkey-selfcheck.sh`, `release-notes-selfcheck.sh` | **Nada a fazer**: a lacuna que a pesquisa supôs não existe — o `release.yml` roda o preflight `release-macos.sh --check` antes de assinar. Os quatro self-checks são de sessão: nenhum workflow os chama |
| Lint de TS determinístico (as duas) | Não | **Adotado na forma mínima** (type check estrito); linter de verdade fica na issue filha |
| Cobertura de `src-tauri` | Não | **Adotado** |
| Auditoria de dependências | Não | **Adotado** |
| Robustez dos gates por caminho | Sim, com testes: 45 casos no `ci-risk-tier-selfcheck.sh` e 13 + 23 + 27 nos três `tests/shell/ci-*.test.sh` | **Verificado e documentado**, sem código novo |

### Type check estrito, sem dependência nova

- `noUnusedLocals` e `noUnusedParameters` entram no `tsconfig.json`. Efeito medido: **17 achados**
  (14 de `noUnusedLocals`, 3 de `noUnusedParameters`), todos mecânicos. Parâmetro prefixado com `_`
  continua aceito, que é a convenção do `tsc`.
- **Não** entra `noUncheckedIndexedAccess`: 120 achados, refatoração não relacionada, escondida atrás
  de um número grande. Fica registrado como o próximo degrau, com o número.
- Os 17 achados são corrigidos removendo o símbolo, não com `// @ts-expect-error`. Um parâmetro que a
  assinatura exige pode ser prefixado com `_`, mas é exceção justificada, não a regra. Um deles
  (`onChange` recebido e ignorado em `ToolchainPanels`) pode ser fiação faltando: se for, a correção é
  ligar a fiação, não apagar o prop — o ticket decide com o código na frente e registra o que achou.
- Nenhum script novo no `package.json`: `bunx tsc --noEmit` já é o comando do CI e do `bun run check`,
  e as flags moram no `tsconfig.json`, então não há como o local e o CI divergirem.
- **Linter de TS fica fora**, com o baseline medido para a issue filha: Biome 2.6.6 com recorte de
  regras de bug real acusa 175 achados, dos quais 138 são `useExhaustiveDependencies` e 5 são
  `useHookAtTopLevel` — **todos os 5 falsos positivos, todos em specs Playwright** (Biome confunde o
  callback do `test()` com um componente React). Sem `useExhaustiveDependencies` sobram 37. Custo:
  1,1 MB de JS + 81 MB de binário de plataforma. Com a config default o número é inútil (596 erros +
  407 warnings, mais 1127 `noReactNativeRawText` disparados por o projeto não ser React Native): a
  config é obrigatória, e é essa config que a issue filha precisa decidir.

### Job `rust-tauri`

- **Job novo, não passo no job existente.** O job Rust é o caminho crítico hoje (3:38–4:21 de uma
  parede de ~4:20); somar ~2–4 min a ele levaria todo PR a ~7 min. Em paralelo, a parede não muda.
  O custo é ~35 linhas de YAML, uma entrada no `needs` do agregador `ci` e um `rust-cache` próprio.
- **Antes do clippy, as duas pastas de resource vazias.** `cargo check --manifest-path
  src-tauri/Cargo.toml` **falha em checkout limpo**: o `tauri-build` valida os resources do bundle e
  `src-tauri/resources/csharp-ls/` e `src-tauri/resources/dotnet/` são vendor dirs gitignored. O job
  as cria vazias (`mkdir -p`), com comentário dizendo que o CI não roda os vendor scripts e que só a
  existência do caminho importa. Não se mexe no `tauri.conf.json` por causa disso.
- **Comandos**: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
  (2,4 s quente aqui, limpo) e `cargo test --manifest-path src-tauri/Cargo.toml` (6 testes, 38,7 s
  quente, limitado por link).
- **Fase informativa** por `continue-on-error: true` no job: o `result` do job vira `success` e o
  agregador `ci` continua verde, então **nada muda no ruleset nem na lista de required checks**.
- **Critério de promoção, escrito**: 5 execuções verdes consecutivas, pelo menos uma delas com cache
  frio, duração estável abaixo de 5 min e nenhum flake. A promoção é remover o `continue-on-error`
  num PR próprio; como o agregador `ci` é o check obrigatório, o job passa a bloquear sem tocar em
  settings do GitHub.
- **Sem script no `package.json`.** O comando vive no workflow e no `AGENTS.md`; um script que o CI
  não chama criaria deriva. O `bun run check` (hook de pre-commit) **não** ganha o clippy do Tauri:
  ele roda a cada commit e o Tauri compila a stack inteira.

### Workflow `dependency-audit.yml`

- **Semanal, não por PR.** Um advisory publicado hoje não deve bloquear o PR de amanhã. O workflow
  não entra no `needs` do agregador `ci`, então não pode bloquear merge nenhum.
- **Gatilhos**: `schedule` (semanal), `workflow_dispatch`, e `pull_request` filtrado por caminho
  (`bun.lock`, os dois `Cargo.lock` e o próprio arquivo do workflow). Motivo do terceiro: `schedule` e
  `workflow_dispatch` só rodam a partir da branch default — sem ele, o PR que introduz o workflow não
  teria como prová-lo, e a #142 exige run registrado. Efeito colateral desejado: a dependência nova é
  auditada no PR que a introduz.
- **Ferramentas**: `bun audit`, que já vem no bun 1.3.9 que o CI fixa (0,4 s medidos, zero dependência
  nova), e `cargo audit` 0.22.2 com versão fixa, rodado duas vezes — na raiz e em `src-tauri`, porque
  são workspaces separados com lockfiles separados.
- **Severidade**: qualquer **vulnerabilidade** falha, em qualquer severidade. Os avisos do
  `cargo audit` (unmaintained, unsound, informational) são impressos e **não** falham — baseline de
  hoje: 1 na raiz (`rustls-pemfile`, RUSTSEC-2025-0134) e 6 em `src-tauri` (glib/zvariant, deps de
  Linux). Nenhum é acionável no macOS, e transformar aviso em gate faria o job nascer vermelho.
- **Falha de infraestrutura é vermelho, nunca "limpo"**: sem `continue-on-error`, sem `|| true`, sem
  cache do advisory-db. Medido: `bun audit` com a rede fora sai 1 com "audit request failed";
  `cargo audit` com banco inacessível e `--no-fetch` sai 1 com "error loading advisory database".
  Como o runner é limpo, ele precisa buscar o banco em toda execução — e se não conseguir, falha.
- **Exceção** é `--ignore` com comentário na mesma linha: o ID, o motivo e a data de revisão. Nada
  **executa** essa data — o lembrete é o run semanal vermelho e a leitura humana. Limitação declarada,
  não promessa.
- **Prova exigida**: um run registrado via `workflow_dispatch` na branch da frente, e um projeto de
  probe **fora do repo** com advisory real (`lodash@4.17.15`) mostrando `bun audit` sair 1 com 5
  advisories. O probe não entra no repositório.

### Item 4 da #142 — robustez dos gates: verificar e documentar

Nenhum código novo. A cobertura que a issue pede já existe e passa hoje (os quatro scripts de CI:
45 + 13 + 23 + 27 casos):

| Critério da #142 | Onde está provado |
|---|---|
| Caminho desconhecido | `scripts/ci-risk-tier-selfcheck.sh`: caminho sem regra vira `medium` |
| Falha de classificação | idem: ref inexistente e diff vazio saem 2, e o job cai para `high`; valor de `Cargo.toml` não parseável classifica `high` direto (`cargo-unparseable-value`) |
| Arquivos sensíveis | idem: workflow, ruleset, `AGENTS.md`, `scripts/release*`/`vendor-*`/`ci-*`, entitlements, `tauri.conf.json`, `trusted.rs`, lockfiles |
| Alteração do próprio gate | `tests/shell/ci-workflow-bootstrap.test.sh`: classificador, gate e agregador são lidos de `origin/<base>` como blob regular e não vazio |
| Mudança do head depois da aprovação | `tests/shell/ci-risk-gate.test.sh`: aprovação em commit antigo, dismissal, changes-requested, sem write access, falha de API |
| Não duplicar #74/#139 | a #139 continua sendo o residual declarado (o YAML do PR roda); exige mudança de ruleset e é decisão do Helder |

**Sem filtro de job por caminho.** Medido: o job Rust é 3:38–4:21 de uma parede de ~4:20, então
filtrar por caminho não economiza nada num PR que toca Rust e cria o risco de pular validação em
silêncio quando o filtro erra. A #142 só autoriza seleção por caminho "se houver ganho medido" — não
há.

### Documentação

- `docs/SPEC-CI-PRACTICES.md` (este arquivo), `docs/evidence/ci-practices.md` com os comandos e os
  números, uma entrada em `docs/decisions.md` e uma linha em `docs/README.md`.
- `AGENTS.md` ganha os dois comandos locais novos na seção **Comandos** e a nota de que o CI não roda
  os vendor scripts do `src-tauri`.

## Testing Decisions

O trabalho é configuração de CI: dois jobs, uma flag de `tsconfig` e um workflow. Não há branch,
parser nem regra de negócio nova, então **não há seam novo e não há teste novo**. Os três seams
reusados:

1. **O run de CI do próprio PR** — o mais alto que existe aqui. É a prova dos dois jobs: `rust-tauri`
   verde (informativo) e `dependency-audit` disparado. A evidência é o link do run no SHA entregue,
   não um teste local.
2. **`actionlint` + `zizmor` no job `governance`**, que já rodam em todo PR e passam a lintar o
   arquivo novo e o job novo — é o seam que pega YAML inválido, permissão larga e injection sem
   escrever teste.
3. **`tsc --noEmit`** no job TypeScript: com as flags no `tsconfig.json`, o fail-antes/passa-depois é
   medido com o próprio comando (17 achados hoje, 0 depois). O repo não tem infraestrutura de DOM e
   não ganha uma por causa disto.

O que **não** se cria, e por quê: nenhum `scripts/ci-*.sh` novo (o workflow chama as ferramentas
direto e quem decide vermelho/verde é o exit code delas, já medido); nenhum `tests/shell/*.test.sh`
novo (não há lógica para cobrir); nenhum script no `package.json` para o `src-tauri`.

Consequência declarada: a prova desta frente é **run de CI + comando local medido**, não teste
automatizado.

## Out of Scope

- Linter de TS (Biome ou ESLint): dependência nova, decisão do Helder. Fica em issue filha com o
  baseline medido.
- `knip`, `cargo-deny`, `cargo-machete`, `cargo-nextest`, `cargo-mutants`, CodeQL, cobertura:
  avaliados na #73 e na #142, nenhum com problema concreto medido que justifique o custo agora.
- Review automatizado por LLM em PR: exige credencial ou serviço pago.
- `noUncheckedIndexedAccess` (120 achados).
- Qualquer mudança em `.github/rulesets/**`, required checks ou settings do repositório.
- `.github/workflows/release.yml` e o `platforms.yml` da #54.
- Path filter de job por caminho.
- Reformatação, refatoração geral e qualquer mudança de produto.

## Further Notes

### Duração medida no run entregue (2026-11-07, `42395785621`, PR #225, `40b7a1f`)

| Job | Duração |
|---|---|
| TypeScript (tsc + unit) | 21 s |
| Rust (fmt, clippy, host tests) | **4:08** |
| Tauri (clippy + tests) — job novo, informativo | 2:41 |
| Governance | 19 s |
| security | 22 s |
| Risk tier | 15 s |
| `ci` (agregador) | 13 s |
| Parede do run | **4:22** |

O `rust-tauri` terminou 1:27 antes do Rust, então a parede continua sendo o job Rust e o job novo não
custou parede: os três runs verdes mais recentes **sem** ele, no mesmo dia, tiveram parede 4:43, 5:07
e 4:49 (Rust 4:26, 4:49 e 4:30). A medição anterior, mais cedo no mesmo dia, dava TypeScript 17–19 s,
Rust 3:38–4:21, governança 21–22 s, security 11–16 s, risk 12–18 s, agregador 12–14 s e parede
≈ 4:20 — a variação do runner é maior que o job novo.

O `dependency-audit` custa ~30 s de ferramenta mais o fetch do advisory-db: **29 s** medidos nos dois
gatilhos (`42395791925` por `workflow_dispatch`, `42395786265` por `pull_request`).

### O que é configuração existente, o que foi executado e o que não foi

- **Executado nesta máquina**: `tsc` com as flags (17), Biome e knip via `bunx` (sem tocar o
  `package.json`), `cargo clippy`/`test` de `src-tauri` com as pastas vazias, `bun audit` (limpo, com
  falha de rede e com advisory real), `cargo audit` nos dois lockfiles, os 4 scripts de CI
  (45 + 13 + 23 + 27 casos).
- **Execução comprovada no runner**: os três runs do SHA `40b7a1f` — os 7 jobs do `ci`, o job novo em
  2:41 e o `dependency-audit` em 29 s nos dois gatilhos.
- **Só configuração existente, não exercitada por esta frente**: os gates de risco e o ruleset.
- **Não verificado**: o `schedule` semanal (só se prova depois do merge na default); o critério de
  promoção do `rust-tauri` (1 de 5 runs verdes); `vite build`, que nenhum workflow roda — residual
  declarado.
