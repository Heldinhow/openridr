# Retro — MVP Rider (F1–F9)

Data: 29/10/2026. Sessão: implementação das fatias F1–F9 na branch local `mvp-rider` (0130761 → 2bb59dd), com migração Mac → VPS Linux arm64 no meio. Fontes: esta sessão, `docs/REVIEW-MVP-RIDER.md`, `docs/implementation-notes.md`, log do OpenCode.

## O que funcionou (manter)

- **Um subagent por ticket, com decisões fechadas e ponteiros** (spec, ticket, decisões, commit anterior). Nove tickets entregues com build/testes e commit próprio; os subagents não reabriram decisão de produto.
- **AC em checkbox + Status com hash** no ticket. Deu para retomar depois de perder o Mac sem reconstruir contexto.
- **Suíte E2E completa ao fim de cada fatia** (quando havia macOS). Pegou duas regressões reais que os specs locais não pegariam (motivo de `git.commit` nas ações/busca após a F6) e expôs um flake (`trust-sdk`).
- **"Não verificado" explícito** em `implementation-notes.md` por fatia, com motivo de ambiente. Nenhuma prova foi inventada.
- **Decisão registrada no momento** (`debug de teste fora`, `commit --only`, tema escuro). O `docs/decisions.md` virou a fonte que os subagents seguintes leram.

## Candidatos de melhoria (ordem de severidade)

1. **O repo não tem guardrail nenhum.** Existe `bun run build` (tsc + vite), mas nada o executa automaticamente: sem CI, sem pre-commit, sem lint. Prova: 5 `eslint-disable` mortos e um comentário em português sobreviveram a nove tickets até a revisão; `messageOf` foi copiado seis vezes. Ação: script `check` (`bun run build` + `cargo fmt --check`, e `clippy -D warnings` quando couber) e um hook de pre-commit; nenhum precisa de dependência nova de UI.
2. **Não existe `AGENTS.md` de repo.** O global é grande e não conhece este projeto; cada sessão redescobre spec, tickets, comandos de verificação e regras (testids congelados, zero dependência nova, macOS-only). Ação: `AGENTS.md` curto, só com ponteiros — `SPEC-MVP-RIDER.md`, `tickets-mvp/`, `decisions.md`, `implementation-notes.md` e o comando `check`.
3. **`playwright.config.ts` fixa `channel: "chrome"`.** Na VPS não há Chrome; cada subagent inventou uma config temporária para rodar specs com Chromium. Ação: ler `PLAYWRIGHT_CHANNEL` (default `chrome`) e documentar que specs dependentes de .NET/macOS são esperados como skip, não como falha silenciosa.
4. **Sem `CODING_STANDARDS.md` para julgamento.** A revisão pegou "três painéis num arquivo" (`git.tsx`), acesso a ação por id com `!`, e mapeamento de erro duplicado — nada disso é mecânico o bastante para lint, mas é repetível. Ação: doc curto para o revisor (um componente por arquivo, acesso a ações via helper, comentários em inglês, uma função de erro por fronteira).
5. **Provedor instável.** Quatro `AI.Error: This API key has expired` e três `ttft_admission_refused` na mesma sessão; um ticket (F9) precisou de duas tentativas e a revisão de spec por subagent nunca completou. Ação: reautenticar `opencode-go` antes de runs longos; manter no prompt de subagent "se o provedor falhar, pare e reporte" (funcionou) e cair para a sessão principal quando o subagent não sobe.
6. **Migração de máquina perdeu trabalho.** O repo chegou na VPS no commit da F6; F7/F8 feitos no Mac foram perdidos e refeitos. Ação: ao migrar, copiar o `.git` inteiro e o tip da branch, e registrar no handoff `HEAD` + tickets concluídos (o `.git` chegou, mas o tip não).

## Débito de código (não é ambiente)

`WorkspaceView.tsx` com 4.770 linhas e 96 hooks, `git.rs` com três painéis e helpers de teste duplicados entre specs. Registrado em `docs/implementation-notes.md` e em `REVIEW-MVP-RIDER.md`; merece fatia própria, não um passeio no fim da branch.

## Aplicado nesta sessão

- Correções da revisão: comentário traduzido, `requireAction`, `messageOf` único (6 arquivos), comentários mortos removidos, `write_json_atomic` compartilhado. Commit `2bb59dd`.
- Review consolidada em `docs/REVIEW-MVP-RIDER.md`.
- Nenhuma mudança de infraestrutura (itens 1–4) foi feita: ficam propostas para a próxima fatia, porque adicionam arquivos de processo e não código do MVP.

## Gate final da auditoria de UI (29/10/2026)

- `bun run check`: passou. Playwright em Chrome headed, `slowMo=300`, com .NET 8 e netcoredbg reais: 18/18 passaram (`run`, `debugger`, `git`, `ui-layout`, `run-configs`, `editor-overflow`).
- Provas observadas: saída normal do Run (`Order A-104`); breakpoint pelo gutter; sessão Debug pausada nele; frames e locals visíveis no painel sem scroll; step into, step out e step over. O spec também confirmou saída/terminação e Stop.
- Capturas/logs desta rodada fora do repo: `/private/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/opencode/openridr-ui-audit-20261029/` (`screenshots/12-run-output.png`, `screenshots/14-dap-paused.png`, `screenshots/14-dap-stepped.png`, `screenshots/14-dap-stopped.png`). Specs de layout cobriram 1600×1000, 1440×900 e 1100×700.
- Funcionou: runner E2E isolado, fixture .NET real e adaptador real permitiram provar o fluxo ponta a ponta. Falhas/skip: nenhum nesta seleção.
- Risco restante: validação é macOS ARM64; Windows/macOS Intel seguem sem prova. Alterações preexistentes em `docs/evidence/`, `docs/tickets-mvp/README.md` e `.scratch/` foram preservadas; sem mudanças de app.
