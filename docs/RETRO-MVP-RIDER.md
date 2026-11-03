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
- Capturas/logs desta rodada fora do repo: `<tmp>/opencode/openridr-ui-audit-20261029/` (`screenshots/12-run-output.png`, `screenshots/14-dap-paused.png`, `screenshots/14-dap-stepped.png`, `screenshots/14-dap-stopped.png`). Specs de layout cobriram 1600×1000, 1440×900 e 1100×700.
- Funcionou: runner E2E isolado, fixture .NET real e adaptador real permitiram provar o fluxo ponta a ponta. Falhas/skip: nenhum nesta seleção.
- Risco restante: validação é macOS ARM64; Windows/macOS Intel seguem sem prova. Alterações preexistentes em `docs/evidence/`, `docs/tickets-mvp/README.md` e `.scratch/` foram preservadas; sem mudanças de app.

## Continuação: gates de execução e ticket 23 (29/10/2026)

- As causas dos stalls de run-config/debugger eram seletores ocultos de ações do workspace; no Run, o evento de click era passado como `requestedConfig`, deixando a execução sem projeto. Ambas foram corrigidas.
- A suíte headed Chrome completa passou 93/93 antes do gate final de elegibilidade do diálogo. Após esse gate, o teste focado de build-failed passou 1/1; a suíte completa não foi repetida depois das últimas mudanças.
- Ticket 23 tem prova headed focada para Problems e rótulos longos de branch/comando/caminho. O `check` continua em andamento por critérios visuais subjetivos da toolbar/Problems; `bun run check` passou.
- Commits locais apenas, sem push. Evidências, scratch e README preexistentes foram preservados. Validação em macOS ARM64; Tauri, WCAG, Windows e macOS Intel não verificados.

## Follow-up: execução após rebuild (29/10/2026, commit `31bf890`)

- Revisão encontrou que Run no diálogo com buffers sujos podia executar o build após Save all, mas não iniciar a configuração selecionada. Corrigido: a configuração selecionada só é iniciada após rebuild bem-sucedido.
- O spec focado `run-configs` passou 4/4 em Chrome headed com `slowMo=300`; `bun run check` passou. Revisão de correção: nenhum finding.
- A suíte completa 93/93 registrada acima ocorreu antes desta mudança; não foi repetida depois dela. Portanto, a cobertura após este commit limita-se ao spec focado e ao `check`.

## Continuação D-12 — prévia que não prende o editor (2026-10-31)

- O diff herdado (efeito de `pointerdown`/Esc + spec) foi validado, não tratado como prova: o spec foi reforçado para hit-testing real (`elementFromPoint`), foco real (`toBeFocused`), sobreposição geométrica do popup, 1440×900 e 1100×700, buffer+disco intactos e Apply em revalidação.
- Red sem o listener (exit 1, popup permaneceu) → green 1/1; `csharp-ls.spec.ts` focado 10/10 (teste de instalação real excluído); `bun run check` e `bun run build` exit 0. A suíte completa não foi repetida.
- Limites (superseded em parte pelo ticket 31: o popup agora é painel docado e não sobrepõe o editor; M7b rename inline segue aberto): na época o popup sobrepunha o centro do editor; a saída é pelo texto alcançável do editor ou Esc com foco nele, não por qualquer clique fora; nativo Tauri não verificado e D-13 não alterado.

## Auditoria final D12 — cancelamento central, stale-error e provas por saída (2026-10-31)

- O guard de `applying` ficou central em `cancelEditPreview`; o Apply verifica sequência + identidade antes de qualquer mutação de sucesso/falha após os `file.read`, então uma leitura antiga não marca a prévia nova como stale nem sobrescreve o status novo.
- Evidência red/green: stale-error red antes da correção (a leitura antiga abortada trocou `Rename preview:` por erro), guard central red no Esc durante Apply → green D12 7/7 e spec focado 16/16. Provas por saída incluem hash imediato, 1440×900 e 1100×700, e 6 cenários independentes de foco+digitação/undo; o foco é assertado antes de qualquer refocus e os 6 cenários passaram 6/6 após esse endurecimento.
- Limite mantido na época (overlay resolvido depois no ticket 31/M7a; M7b aberto): só texto alcançável/Esc, sem acesso ao texto coberto; nativo Tauri não verificado. Auditoria independente terminal aprovou D12 no intervalo `2a054ca..7d5b62e` por leitura+logs, sem nova execução.

## D13 — Trust popover fecha no sucesso (2026-10-31)

- O fechamento usa o boundary de sucesso já existente no `TrustControls.onChange` do toolbar; pending e erro não chamam `onChange` e continuam visíveis. O foco volta ao summary ao fechar.
- Red 3/3 (grant/retry deixavam o popover visível) → green 3/3; geometria/hit-testing em 1440×900 e 1100×700; foco no summary seguido de Enter/Escape reais.
- Cleanup D13 usa snapshot `workspace.state` + `realpath` da solução e só fecha a cópia própria; posse/close incerto retém e reporta. Partial-open red 2/2 com cleanup legado → green 2/2; cenário retained deixa a cópia alvo e o workspace de outra cópia do teste intactos.
- Regressões finais: gates 8/8 (D13 6 + D10 + Restricted), restore 2/2, check standalone exit 0; build não repetido com produto inalterado. Logs em `/tmp/openridr-d13-final-review/`.
- Hook: **check standalone verificado; hook histórico não comprovado**. O `.githooks/pre-commit` versionado é 100755, mas o `core.hooksPath` histórico aponta para o root sem `pre-commit`; este followup usa `git -c core.hooksPath=.githooks` e trace em `/tmp/openridr-d13-final-review/hook-trace.jsonl`.
- **Auditoria independente terminal: APROVADO** (Standards e Spec, 0 findings) no intervalo fixo `6de4bea..1c2eb25`, por leitura + evidência registrada, sem novas suítes; P2 partial-open/unknown-state e hook contemporâneo fechados em `1c2eb25`. Isto fecha o **batch D12/D13**; D12 permanece aprovado desde `7d5b62e`. Receipts em `/tmp/openridr-review-1c2eb25-v2qjn/`. Não é aceite global.
- Limites: cleanup aceito só no host E2E exclusivo/serial, 1 worker, sem outro ator; snapshot->close não é transação/atomic ownership. Erro injetado é boundary de UI, não falha real de host/rede; nativo/Windows/Intel/full suite não verificados. M7/overlay permanece não resolvido.

## 2026-10-31 — M7a + prova nativa

### O que funcionou
- **Isolamento do app de prova:** `OPENRIDR_HOME` apontando para um home descartável, cópia da fixture em repo git próprio e um remote bare local. Nada tocou o projeto real, o app pessoal ou o estado de trust do usuário.
- **Denominador fixo e regra estrita:** 33 itens numerados, com "provado" só quando o escopo inteiro do item foi observado no `.app`. Sub-itens observados contam como parcial.
- **Evidência atribuída por SHA de binário:** cada captura diz qual SHA prova o quê (`34a87b38`, `35038ee6`, `810e70a8`, `4943eba3`). Um delta de CSS não reabre provas anteriores; só é declarado.
- **Rebaixamento honesto:** o item 11 foi rebaixado de provado para parcial quando o escopo se mostrou maior que o sinal (`Restore succeeded`). A manchete caiu de 1/22 para 0/23 sem drama.
- **Dois canais de interação:** AX/agent-desktop com refs qualificadas para ações semânticas, e teclado para o que a webview não expõe.

### O que doeu
- **Falso positivo do Welcome (NSOpenPanel órfão):** um painel modal de `openAndSavePanelService` deixado por um Open anterior bloqueava a webview. `screencapture -l <winid>` não mostra painel, então parecia clique que não funcionava. Ficou como defeito aberto por um ciclo inteiro de sessão.
- **Captura de tela inteira só para diagnóstico:** é a única forma de enxergar painéis fora da janela, mas mostra o desktop inteiro. Quem fez a captura precisa cortar antes de publicar.
- **Popup de branches com refs de outra cópia (r4):** não reproduzido depois (r5). Causa não confirmada; a hipótese mais provável é captura de outra janela do mesmo bundle id. Ficou registrado como não-defeito com evidência, não como encerrado.
- **Duas leituras erradas na contagem:** a narrativa "14/33" somava parciais e provados. Reconciliar a matriz custou três rodadas de revisão.
- **Privacidade descoberta tarde:** prompt zsh com usuário/host apareceu em capturas que passaram por revisão de conteúdo visível. OCR achou depois. Havia 96 PNGs rastreados com identificadores, dos quais a maioria é anterior à sessão.
- **E2E de 1100×700 cego ao corte:** o teste de layout não abria o painel Problems, então o corte do preview só apareceu depois da prova nativa.
- **Consumo de contexto por imagens:** cada captura reduzida custa muito. Sem limite de uma imagem por verificação, a sessão esgotou contexto várias vezes.
- **Acerto de coordenadas:** clique por coordenada de ponto errou com frequência (escala 2×, janela sem foco, faixas que mudam de altura com Problems aberto).

### Ações de melhoria (pit of success)
1. **Cenário e2e com tool window aberta:** o teste de layout 1100×700 deve abrir o painel Problems e verificar que o preview de rename, de code actions e de Find Usages continua com Apply/Cancel e 1ª linha visíveis. Hoje isso só é checado no `.app`.
2. **Gate de OCR antes de commitar evidência:** um script que roda tesseract sobre PNGs novos e recusa commit se achar `usuário@host`, `/Users/`, `/private/var/folders` ou o nome de autor git. Ficaria no mesmo lugar dos outros hooks.
3. **Remover prompts do terminal nas capturas por construção:** o recorte deve começar abaixo da linha do prompt, não depender de OCR depois.
4. **Descarte de buffers de teste automatizado:** o fluxo "fechar com buffer sujo" deveria ter um atalho de sessão de teste que descarta sem diálogo, para não precisar de clique de coordenada.
5. **Registrar SHA por evidência desde o primeiro commit:** a regra de atribuição funcionou, mas a primeira versão do log não tinha SHA. Modelo: `SHA + PID + janela + comando` no cabeçalho de cada log.
6. **Limite de imagens por verificação:** uma captura por item, reduzida, com OCR no mesmo passo. Captura de diagnóstico fica fora do repo.

### Privacidade e política de push (pendente, decisão do Helder)
- **96 PNGs rastreados com achados de OCR** (categorias: prompt com host, `/Users/<nome>`, `/private/var/folders`, nome de autor). A lista completa, com commit introdutor de cada arquivo, está em `docs/evidence/logs/privacy-redaction-2026-10-31.md`. Nenhuma imagem pré-existente foi alterada.
- **Histórico local contém blobs com host e usuário** (ex.: native-19 em `b2f7d78`, native-20 em `8b223c6`). **Não publicar o branch sem filtrar esses blobs** (`git filter-repo` ou reescrita controlada) ou publicar só uma cópia limpa. A escolha é do Helder; nada foi executado.

### Agent-desktop e AX
- **Destravou:** fechar aba suja (diálogo "Close modified file?" → "Discard and close") e as ações de rename, code actions e Find Usages por refs qualificadas, sem coordenadas.
- **Não destravou:** completion. `press ctrl+space` foi entregue sem popup, e `type` no editor retornou `ACTION_FAILED` com o buffer inalterado. A webview do editor não expõe ref de texto nem caret.
- **Regra adotada:** ação só com ref vigente, retry só com `disposition.retry: safe`, leitura de estado depois de toda ação `delivered_unverified`.

### Lacunas e limites
- **M7b (campo inline sobre a palavra com range)** continua aberto; não implementado.
- **Itens sem prova nativa completa:** 2 (Revoke com Run ativo), 5 (salvar e fechar aba com Keep editing), 7 (completion), 8 (rename 1440 no SHA final), 9 (invalidação antes de escrever), 12 (erro navegável), 16 (Rerun failed, cancelar, TRX, MTP), 21 (Environment vazio, `.env` válido, troca de Configuration/TFM).
- **Windows e macOS Intel não foram executados.**
