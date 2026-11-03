# Retro — navegabilidade do repo (2026-11-03)

Escopo: as 10 sessões de topo mais recentes deste repo no OpenCode/OpenChamber (mais seus subagentes), lidas em `~/.local/share/opencode/opencode.db` (`session_v2` + `session_message`). Não há sessões de Claude Code para este repo (`~/.claude/projects` só tem a pasta do worktree `alpha-release`, fora da amostra). Contagens extraídas das chamadas de ferramenta; "busca/leitura via shell" = comandos `cat/sed -n/grep/rg/ls/head/wc`.

## Evidência por sessão

| Sessão | Chamadas | Read | Grep/Glob | Shell (busca/leitura) | O que buscou / onde perdeu tempo |
|---|---|---|---|---|---|
| `e5fc` Release alpha.5 | 90 | 2 | 0 | 79 (65) | `docs/RELEASE-ALPHA.md`, `install-app.md`, onde a versão está; `node_modules` ausente no worktree (2 saídas vazias); `cargo test` com 2 filtros → `unexpected argument`; 1 falha de `git commit` (arquivo novo `docs/release-signing.md`) |
| `e614` Abrir pelo Finder | 473 | 28 | 3 | 324 (290) | Abertura: ~10 chamadas só para achar skills e `docs/`; `implementation-notes.md` 39× e `decisions.md` 32× (via `sed -n`/`cat`); 1 falha de `git commit` (spec e tickets novos) |
| `e616` Release alpha.4 | 69 | 16 | 0 | 46 (39) | `docs/RELEASE-ALPHA.md` 7×; sem falha de navegação relevante (fluxo repetido da alpha anterior) |
| `e61d` Médios: editor | 736 | 195 | 85 / 0 | 357 (272) | **10 falhas de `git commit`** (`-m` depois do `--`; arquivos novos); releitura de `.scratch/logs/e2e-*.log` por faixas de linhas (`sed -n 8071,8270p`…) |
| `e623` Médios: debug/run | 843 | 211 | 106 / 3 | 354 (308) | **`docs/README.md` lido 5×** (era a proposta de UI de 28/10, “aguardando aprovar as telas”); `decisions.md` 19×; 6 falhas de `git commit` |
| `e62c` Médios: navegação | 1.253 | 416 | 136 / 15 | 511 (397) | Maior consumo: 416 reads + 136 greps, dominados por `WorkspaceView.tsx`; `grep` em `node_modules/@codemirror` vazio; `decisions.md` 17×; 5 falhas de `git commit` |
| `e633` 4 altos devflow | 625 | 157 | 73 / 5 | 304 (258) | `ls`/`find node_modules` vazios (2×); 8 falhas de `git commit` (`--only -- … -m`) |
| `e637` QA devflow | 321 | 39 | 2 | 268 (253) | `.scratch/host-home` inexistente (1 `ls` vazio); `.scratch/qa-shots2` adicionado ao index e removido depois; 2 falhas de `git commit` |
| `e66f` Alfa 3 | 558 | 148 | 37 / 4 | 282 (235) | `ls`/`find` em `node_modules/playwright` vazios (2×); `implementation-notes.md` 25×, `evidence/alpha3-debug-trust.md` 25×; 2 falhas de `git commit` |
| `e68c` Run/Debug auto-build | 739 | 300 | 73 / 6 | 223 (171) | `cargo check -p host` (nome de pacote errado); `decisions.md` 22×; 3 falhas de `git commit` (inclui evidência nova) |
| **Total** | **5.707** | **1.512** | **515 / 33** | **2.748 (2.288)** | **38 falhas de commit** |

Arquivos mais lidos (Read + `sed -n`): `src/WorkspaceView.tsx` **403×** (5.229 linhas, um único componente), `crates/host/src/lib.rs` **198×** (7.704 linhas), `src/styles.css` 58×, `src/host/client.ts` 36×, `crates/host/tests/host.rs` 47×, `docs/decisions.md` 43×.

## Achados, por severidade

1. **`git commit --only` falhou 38 vezes em 10 sessões** (`pathspec '-m' did not match` e `pathspec '<arquivo novo>' did not match`). A regra do `AGENTS.md` dizia só `git commit --only -- <paths>`. Reproduzi em repo temporário: `-m` depois de `--` vira pathspec, e arquivo novo exige `git add -N` antes. Cada falha custa ≥2 chamadas (erro + repetição).
2. **`docs/README.md`, a porta de entrada de `docs/`, era a proposta de UI de 28/10** (“nenhum produto implementado… aguardando Helder aprovar as TELAS… não criar tickets”). Lido 5× numa sessão; `SPEC.md`, `wireframes.md`, `decisions.md` e `index.html` apontavam para ele. Não havia índice de `docs/` (40 `.md` na raiz + 8 subpastas de tickets/evidência), então cada sessão reconstruía “qual spec/ticket/retro pertence a esta feature” com `ls`+`cat`.
3. **Sem mapa do código.** 403 leituras de `WorkspaceView.tsx` e 198 de `lib.rs`, sempre por faixa de linhas descoberta por tentativa. Nenhum doc dizia que `dispatch` é uma sequência de `if method == "x.y"`, que os testes de `lib.rs` começam em `mod tests` (~linha 7.020), nem como achar a dona de um `data-testid`.
4. **Worktree nova sem `node_modules`**: 5 das 10 sessões gastaram chamadas em `ls/grep/find` vazios ou comando que falhou; o `AGENTS.md` só dizia `bun install` de forma genérica.
5. **Nomes/flags de cargo errados**: `-p host` (o pacote é `openridr-host`) e `cargo test <f1> <f2>` (só aceita um filtro antes do `--`).
6. **Falhas pré-existentes reaprendidas a cada sessão.** A lista (find-usages, 2 de run-configs, 2 de tests.spec, `file_access_is_confined_to_the_workspace`) só existia em 5 retros/specs; sessões de teste completo (14–18 min) a redescobriam, e a sessão de release recebeu a lista no prompt.
7. **Docs históricos sem aviso**: `release-decision.md` (“NÃO APROVADO — RELEASE BLOQUEADO”, 28/10, já superado por 4 alphas), `tickets/README.md` (18 tickets `ready-for-agent` já implementados), `index.html`/`wireframes.md`. Numeração de tickets reinicia por pasta (`tickets/01` ≠ `tickets-mvp/01`) sem aviso.
8. **Spec/tickets de Welcome + Trust vivem em `.scratch/welcome-trust/`**, pasta que o resto dos docs trata como lixo de sessão (182 arquivos rastreados, 95 deles `ui-audit/text`).

## O que mudou (só docs e orientação; nada em `src/`, `crates/`, `src-tauri/`)

- `docs/MAPA.md` (novo): mapa do código (UI, host, pastas) + tabela “tenho X → rode Y” (testid, método do host, wrapper TS, atalho, decisão, ticket).
- `docs/README.md` (novo índice): o que é vivo × histórico, tabela feature → spec/tickets/retro/evidência, convenção de nomes para trabalho novo, aviso sobre numeração de tickets.
- `docs/PROPOSTA-UI-2026-10-28.md` (era `docs/README.md`, via `git mv`) com banner “histórico”; links em `SPEC.md`, `wireframes.md`, `decisions.md`, `index.html` atualizados.
- Banners “histórico/superado” em `release-decision.md` e `tickets/README.md`.
- `AGENTS.md`: ponteiros para MAPA/índice; receita correta de `git commit --only`; `bun install` obrigatório em worktree nova; nomes dos pacotes cargo e filtro de teste; lista de falhas conhecidas; `scripts/` e `.scratch/` na árvore.
- `README.md` da raiz: de uma linha para 4 ponteiros.

## Não feito (decisão sua)

- **Quebrar `WorkspaceView.tsx` e `lib.rs`** (código de produto, fora do escopo). É a maior alavanca restante: o `useExecutionFlow` já mostra o caminho. O mapa só reduz a busca, não o tamanho do trecho lido.
- **Guardrail para o commit**: um script `scripts/commit-only.sh` (faz `add -N` + `commit --only`) eliminaria a classe de erro 1 de vez; docs só reduzem.
- **`docs/install-app.md` ficará desatualizado** quando a alpha.5 (assinada/notarizada, em andamento em outro worktree) entrar: ele diz “sem assinatura nem notarização”. Atualize junto com `RELEASE-ALPHA.md`.
- **`.scratch/`**: 182 arquivos rastreados (capturas, JSON de auditoria); a retro `RETRO-MVP-RIDER.md` já aponta 96 PNGs com dados pessoais no OCR. Considere mover o que for evidência para `docs/evidence/` e parar de rastrear o resto.
- **Fora do repo**: a sessão `e614` gastou ~10 chamadas procurando skills (`grill-with-docs`, `to-spec`) em `~/.claude` e `~/.config/opencode`; vale uma linha no `AGENTS.md` global dizendo onde estão.

## Estimativa de ganho (não medida; hipótese a conferir nas próximas sessões)

- Eliminável quase por completo: ~76 chamadas de retrabalho de `git commit` (38 falhas × ≥2) e ~15–25 de `node_modules`/pacote/filtro errados — ≈ **1,5–2% das 5.707 chamadas** das 10 sessões, quase todas curtas.
- Provável: orientação inicial de cada sessão (~10–20 chamadas de `ls`/`cat docs/…`) cai para 2–3 com índice + mapa; em 10 sessões ≈ 100–150 chamadas e o risco de ler doc obsoleto como verdade.
- Incerto: leitura de `WorkspaceView.tsx`/`lib.rs` (≈600 leituras). O mapa e as receitas de `grep` devem cortar tentativa e erro, mas só a divisão dos arquivos reduz o volume por leitura.
- Ordem de grandeza total: **~4–5% das chamadas** com boa confiança (retrabalho + orientação), chegando a ~10% se o mapa reduzir a tentativa e erro nos dois arquivos grandes; mais uma fração do contexto lido em docs. Nenhuma linha de produto mudou. Reavalie numa retro depois de ~10 sessões com estes arquivos.
