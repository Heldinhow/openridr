# Retro — welcome-trust (tickets 01–06)

Data: 01/11/2026. Branch `feat/welcome-trust`, `2d242ab..415da7d` (6588b6b grill/spec/tickets → 415da7d resolução do review). Fontes: `SPEC-WELCOME-TRUST.md` e `tickets-welcome-trust/` (movidos de `.scratch/welcome-trust/`) (spec, issues), `docs/decisions.md` (2026-11-01), `docs/implementation-notes.md`, `docs/REVIEW-WELCOME-TRUST.md` (com Resolução), `docs/evidence/welcome-trust/VERIFICATION.md`, `git log`.

## O que funcionou (manter)

- **Grill → spec → 6 tickets antes de codar.** As decisões (uma pergunta por projeto, modal estilo Rider, New/Clone fora) ficaram em `decisions.md` e nenhum subagent reabriu produto. A exceção ao contrato de `data-testid` foi decidida de antemão, então a migração dos ~30 specs foi mecânica.
- **Um commit por ticket**, convencional, com `git commit --only`. O histórico (`b4b46b3` … `9dbbb89`) é legível e o review conseguiu mapear achado → ticket.
- **Review estático separado.** Achou 3 majors reais (TOCTOU em `trust.set`, Cancel deixando rastro, Cancel destruindo o workspace anterior) que os testes verdes dos tickets não pegaram. Todos os 14 achados têm status na seção Resolução.
- **Verificação real em Chrome visível** (ticket 06) com host e app home temporários: o app real do usuário não foi tocado, e o roteiro de 7 passos virou evidência (`v01…v07d`).
- **"Não verificado" explícito** em cada nota de ticket e no `VERIFICATION.md`. Nada foi declarado sem prova (ex.: o baseline das falhas de cargo ficou como "não provado").
- **E2E um spec por vez** evitou colisão de portas/fixture e deu resultado confiável (47 testes no HEAD de verificação).

## O que atrasou ou surpreendeu

1. **Cancel deixando rastro só foi visto no review (e de passagem na verificação visível).** O ticket 03 dizia "Cancel volta à welcome sem abrir", mas `recent::record` roda dentro de `workspace.open`, antes da decisão. O spec do ticket 03 afirmava só "sem workspace aberto", não "sem efeito colateral". A observação apareceu em `VERIFICATION.md` como "fica para decisão do dono" em vez de virar bug na hora. Custo: dois commits de correção (`75d96e4`, `7b404c5`) e uma decisão nova.
2. **TOCTOU e Esc durante `busy`** (achados 1 e 3): erros de concorrência em fronteiras que o E2E sequencial não exercita. Só leitura de código achou.
3. **Subagent do ticket 04 terminou sem relatório.** O estado só foi recuperado olhando o diff e o commit `63e4e6b`; a nota de implementação do 04 ficou com a lista de verificação sem contagem de cargo completa. O contrato "devolve ≤15 linhas" não tem como falhar visivelmente.
4. **Hosts E2E órfãos.** `trust-prompt.spec.ts` e `welcome.spec.ts` sobem `cargo run` em `hostPort+1/+2`; `server.kill()` mata o `cargo`, não o `openridr-host-http`. O filho segue na 4318/4319 com `OPENRIDR_HOME` antigo e pode servir estado velho na rodada seguinte. Documentado, **não corrigido**.
5. **Falha de cargo por profundidade de caminho.** `file_access_is_confined_to_the_workspace` usa `../` ×6 e depende de onde o checkout está: falha em `~/.local/share/opencode/worktree/...`, passa em `/tmp`. Cada ticket redescobriu e re-explicou a falha ("baseline?") em vez de o teste ser robusto. Pior: o baseline no mesmo caminho profundo não pôde ser provado (a branch ocupava o worktree).
6. **`evaluation_returns_the_real_msbuild_model...`**: falhou 3/3 no baseline limpo, passou sempre no HEAD. Causa (fixtures não restauradas em worktree novo) não investigada; a "intermitência" citada nunca foi reproduzida.
7. **Rider.app não capturado.** Abri-lo reabriria o último projeto do usuário e mexeria na config. A referência visual ficou só nas capturas antigas e nos tokens; o "estilo Rider" foi julgado de memória.
8. **Evidência desatualizada.** `v02`/`v07a` são de antes da correção do Cancel, e `.scratch/.../verify-visible.mjs` foi commitado e depois removido (achado 6), deixando um log sem o script que o gerou.
9. **`messageOf` copiado de novo** (achado 5), apesar de o AGENTS.md pedir uma função por fronteira e de o retro anterior já ter apontado isso.

## Lições

- Um ticket de UI que diz "Cancel/fechar/desfazer" precisa de AC de **efeito colateral zero** (estado de disco, histórico, workspace anterior), não só de estado visual.
- Operações que decidem e aplicam em dois passos (`set` + `grant`) são suspeitas por padrão: o review deve procurar "lock solto entre dois lugares".
- Observação estranha durante verificação manual é bug até prova contrária; "decisão do dono" só vale se virar entrada em `decisions.md` no mesmo dia.
- Falha de teste "de ambiente" precisa de um dono e de uma correção, não de uma nota repetida em cada ticket.
- O que o ambiente não deixa abrir (Rider.app, VoiceOver, Tauri) continua "não verificado" ciclo após ciclo: ou se cria um jeito de verificar, ou se aceita e para de listar como pendência.

## Sugestões para o próximo ciclo

**Processo**
1. No template de ticket, acrescentar a linha obrigatória "Efeitos colaterais de cancelar/falhar" com AC de arquivo/histórico/estado anterior inalterados.
2. Prompt de subagent de ticket: "a última mensagem é o relatório; sem ele o ticket não conta como feito". A sessão principal confere `git log -1` + nota em `implementation-notes.md` antes de seguir.
3. Observação anômala na verificação visível entra como ticket/bug no ato (`.scratch/<feature>/issues/NN-*.md`), nunca só como texto em `VERIFICATION.md`.
4. Rodar o review **por ticket de risco** (host/concorrência: 01, 03), não só no fim da branch.

**Testes**
5. `file_access_is_confined_to_the_workspace`: montar o alvo a partir de um diretório temporário conhecido (ou calcular os `../` pela profundidade real) para passar em qualquer checkout. Fecha a falha recorrente.
6. Helper único de E2E para host isolado que mata o **grupo de processos** (`spawn` com `detached` + `process.kill(-pid)`) ou usa o binário pronto em vez de `cargo run`. Antes de subir, falhar se a porta já responde.
7. Spec de corrida mínima para `trust.set` vs `workspace.open` no host (teste Rust com duas threads).
8. Investigar `evaluation_returns_the_real_msbuild_model...` em worktree novo: ou restaurar a fixture no setup do teste, ou `skip` com motivo explícito.

**Skills / AGENTS.md**
9. AGENTS.md: acrescentar uma linha em "Comandos" — "worktrees fundos quebram `file_access_is_confined...`; rode cargo em `/tmp` ou veja `docs/RETRO-WELCOME-TRUST.md`" (apenas enquanto o item 5 não sair), e "mate hosts órfãos: `lsof -ti tcp:4317,4318,4319 | xargs kill`".
10. Check mecânico para `messageOf`: grep no pre-commit (ou regra de lint) que falha com `function messageOf` fora de `src/host/client.ts`. Já apareceu em dois retros.
11. Nota de verificação visual: `.scratch/<feature>/` não entra em commit (adicionar ao `.gitignore`), e o script de verificação fica em `tests/` se o log for evidência.
12. `cargo fmt --check`/`clippy` seguem fora do `check`: decidir numa fatia própria, em vez de deixar o `AGENTS.md` pedir para "formatar o que tocar".

## Dívida aberta

- **Achado 12 (nit, perf):** `trusted::get` relê e parseia `trusted.json` a cada snapshot e a cada linha de `recent.list` (até 30×). Sem impacto hoje; ler o mapa uma vez por `list` se a lista crescer.
- **Achado 14 (nit):** read-modify-write de `trusted.json`/`recent.json` entre processos (app + ponte dev) pode perder uma gravação. A escrita atômica evita corrupção, não perda. Limitação registrada em `implementation-notes.md`.
- **Hosts E2E órfãos** na 4318/4319 (`trust-prompt`/`welcome` specs) — sugestão 6.
- **Falha de cargo por profundidade de caminho** e `evaluation_returns_...` não investigada — sugestões 5 e 8.
- **Não verificado:** Tauri real (janela nativa, ⌘O nativo, só Ctrl+O na ponte), leitor de tela (VoiceOver/NVDA; foco/ARIA só por asserts), Rider.app (referência não reaberta), aviso de SDK ausente (sem spec), viewport <900px, menu ⋯ na última linha de lista longa, Windows e macOS Intel.
- **Fora do escopo por decisão:** New (`dotnet new`) e Clone (`git clone`) na welcome; só Open tem suporte real. Só tema escuro. Fonte Inter segue sem arquivo embutido.
- **Efeito do achado 13:** entradas legadas de `recent.json` agora gravam `"openedAt": null` na próxima escrita.
