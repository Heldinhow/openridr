# Retro — Debug: explicar a sessão que termina sem parar (#46)

Sessão filha do orquestrador, worktree `feat/issue-46-debug-no-breakpoints`. Escopo entregue: #76 (host) e #78 (painel), spec `docs/SPEC-DEBUG-NO-STOP.md`, evidência `docs/evidence/debug-no-stop.md`.

## O que funcionou e deve continuar

- **A tabela-verdade na spec antes do código.** Escrever os sete estados de fim (parou, exit desconhecido, mute, breakpoints não atingidos, filtro de exceções, run-to-cursor, cancelado) transformou o review em conferência: os dois achados altos/médios saíram direto dela. O esforço de grill que a precedeu foi o que fixou a fronteira (host dono da regra, só `terminated`, mute com frase própria).
- **Um contrato só para o fato e para a frase** (`noStopExplanation` no snapshot): o painel virou render-only, e o teste de host pôde cobrir a regra inteira sem adapter real nem UI.
- **Falha-antes medida revertendo só a atribuição do campo**, sem `git stash` — barato e não mexe no index.

## Achados de ambiente (ordenados por custo)

1. **O E2E de Debug inteiro está quebrado neste HEAD, e não está na lista de falhas conhecidas.** Na ordem do custo de discovery: perdi ~10 min até entender que `prepareDebugCopy` falha em `debug.install` porque o host de E2E roda com bundle falso e a resposta é `invalid_state: netcoredbg is bundled with OpenRidr`. O mesmo teste do `HEAD` (linha 192, intocada) falha igual, então é pré-existente; o `AGENTS.md` manda tratar toda falha fora da lista como minha.
   *Melhoria*: acrescentar a linha em `## Falhas conhecidas (não são suas)` — "debugger.spec.ts: o helper de setup chama `debug.install`, recusado pelo adaptador embutido; todo o spec falha" — e deixar a correção na frente #43. Um `grep` no arquivo de falhas conhecidas é o que um agente faz primeiro.
2. **`bun run tauri dev` não abre em worktree limpa.** O build do Tauri falha em `resource path 'resources/dotnet' doesn't exist`, e mesmo vendorizando, os overrides de dev (`OPENRIDR_BUNDLED_NETCOREDBG`, `OPENRIDR_BUNDLED_CSHARP_LS`, `OPENRIDR_BUNDLED_DOTNET_ROOT`) são a única forma de o app em dev achar o par embutido — `resources_root()` resolve `<exe>/../Resources`, que não existe fora de um `.app`. Nada disso está no `AGENTS.md`; a informação está só no cabeçalho de `crates/host/src/bundled.rs` e em scripts de vendor.
   *Melhoria*: um `scripts/dev-app.sh` que recebe os caminhos vendorizados e exporta os três overrides (ou uma linha no `AGENTS.md` apontando para os três nomes). Remove ~20 min de tentativa e erro por sessão que precise validar UI no app real.
3. **`agent-desktop` não digita em campo do WKWebView.** `set-value` e `type` no campo de caminho do Welcome voltam `delivered_unverified` com o valor vazio, e o clique em "Open" com o campo vazio abre um `choose file` nativo que deixa o app em "Opening…" para sempre (o `osascript` fica esperando). O caminho que funciona é abrir por argv encaminhado para a instância viva (`src-tauri/target/debug/openridr <path>`, single-instance plugin), e depois o Trust prompt aparece normalmente.
   *Melhoria*: registrar esse caminho na skill de verificação (`verify-openridr`) e no `AGENTS.md`, junto do aviso de que o seletor nativo é a armadilha que trava o app.
4. **`Data::breakpoints` guardava entrada vazia por fonte.** Remover o último breakpoint deixava `{"Program.cs": []}`, o que parece "tem breakpoint" para qualquer leitor que não esvazie a lista. Corrigido na fonte (`set_breakpoints` remove a entrada) e o leitor da explicação continua contando linhas, não chaves. A causa raiz foi um invariante não escrito.
   *Melhoria*: tratar "mapa com listas" como invariante de tipo/teste no host — um teste por mapa que possa ficar com chave vazia custa menos que o achado do review.
5. **Meu próprio tropeço:** ao inserir o teste novo no `debugger.spec.ts` com um `edit` grande, substituí o corpo do teste vizinho (o condicional). O `git diff` do arquivo inteiro pegou antes do commit. Fica o hábito: depois de editar teste grande por `edit`, ler `git diff <arquivo>` inteiro, não o stat.

## O que ficou fora e por quê

- "Breakpoints setados mas nunca atingidos" (condição falsa, breakpoint não verificado): diagnóstico diferente, fora de escopo por decisão do grill.
- Frase sobrevivendo à selagem por troca de workspace: deriva dos fatos e a selagem não limpa, mas não foi provada no app.
- Windows e macOS Intel: não executados, como no resto do repo.
