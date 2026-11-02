# QA do fluxo de desenvolvimento — 2026-11-02

Branch `qa/devflow` (a partir de `main`, 0.3.0-alpha.3, commit `cad7e85`). Nenhum código do produto foi alterado.

## Resumo

- O ciclo central funciona com saída real: editar → Build (erros clicáveis, com sublinhado) → Run → corrigir; Run configurations; testes (descobrir, todos, um, só falhos); debug com netcoredbg (breakpoints, condicional, step, watches, async, exceção, restart). Os achados 2 e 3 da rodada anterior seguem corrigidos.
- Três pontos **bloqueiam o dia a dia**: o editor não rola até a linha atual do debug; `⌘U`+letra (atalhos de teste) e `⇧⌘K`/`⌃D` são engolidos pelo CodeMirror e ainda alteram o buffer; solução com vários consoles deixa Build/Run/Debug desabilitados até escolher o startup project num select escondido.
- Faltam ações que um dev de Rider/VS Code espera: fechar aba, trocar de aba, Back/Forward, Go to Line, Format, Organize usings, Find in Files, split, hover de variável no debug, debug de teste, logpoint.
- Host estável: sessão de debug de ~11 min (~280 ações) com host a 1,9 % de CPU média e ~12 MB RSS, sem vazamento e sem processos órfãos após Stop.
- Não foi possível testar o app Tauri (acessibilidade/gravação de tela negadas ao `agent-desktop`); tudo foi visto no Chrome via `host:http` + Vite.

## Como foi testado

- Três instâncias isoladas (host-http + Vite, portas 4411–4413/5411–5413, `OPENRIDR_HOME` em `.scratch/host-home-{A,B,C}`; a home real nunca foi tocada), cópias de `OrderDesk` e projetos criados em `.scratch/work-*` (solução com 4 consoles + lib, app de debug com modos basic/loop/throw/async). Direção com Playwright e `keyboard.press` com modificadores de macOS. csharp-ls e netcoredbg 3.2.0 instalados pelo próprio app.
- Notas completas por área: `.scratch/qa-notes/{A,B,C}.md` (não versionadas; este doc é o resumo). Amostras de CPU/memória: `.scratch/qa-notes/B-ps-samples.csv`.
- Screenshots em `.scratch/qa-shots2/` (as citadas abaixo; ver "Limitações").

## Achados

Classe: **[B]** bloqueia o dia a dia · **[I]** incômodo.

### Alto

**1. [B] Debug: o editor não rola até a linha de execução atual** — bug
- Passos: Program.cs com editor no topo, breakpoint na chamada de `Compute`, Debug, F7 (Step Into) até entrar no método (~linha 30).
- Esperado: editor rola e mostra a linha amarela. Obtido: a linha atual fica fora da tela. Idem em parada por exceção e no lançamento com o editor rolado. Só rola ao clicar num frame ou Run to Cursor. Reproduzido 3×.
- Causa provável: `CodeEditor.tsx` decora `.cm-line-current` mas não faz `scrollIntoView`; só `navigateTo` (`WorkspaceView.tsx` ~l.1173) rola.
- Shot: sem imagem (recaptura não concluída); reproduzido 3×.

**2. [B] `⌘U` + letra (Run all / Debug / Rerun failed) não funciona com foco no editor e digita a letra no buffer** — bug
- Passos: abrir `OrderTests.cs`, clicar no editor, `Meta+U`, `L`.
- Esperado: roda todos os testes (é o que o Quick List anuncia). Obtido: nada executa e o buffer vira `lusing …;` (aba `*`). Fora do editor o chord funciona. 3×.
- Causa provável: `basicSetup` (`CodeEditor.tsx:771`) liga `Mod-u` a `undoSelection`; o handler de chord em `WorkspaceView.tsx:2965-3018` sai em `event.defaultPrevented`.
- Shot: `A-19-chord-L.png`.

**3. [B] `⇧⌘K` (Git Push) e `⌃D` (Debug secundário) são engolidos pelo CodeMirror** — bug
- Passos: foco no editor, `⇧⌘K` → apaga a linha atual, Push não abre (2×). `⌃D` → `deleteCharForward`, debug não inicia (1×; `⌃R` funciona).
- Causa provável: mesma do item 2 — o keymap global não tem precedência sobre o `defaultKeymap`.
- Shot: `C-06-cmdshiftk.png`.

**4. [B] Solução com mais de um projeto console: Build, Run, Debug e seletor de Run configuration ficam desabilitados** — experiência
- Passos: abrir solução com 4 consoles, confiar, olhar a toolbar.
- Esperado: Build da solução disponível e escolha do que rodar na toolbar (Rider). Obtido: tudo desabilitado ("More than one console project; choose the startup project explicitly."). A única saída é o select "Startup project" na aba Build do painel inferior; `⌘F9` também não responde. O diálogo de Run configurations está desabilitado.
- Causa provável: gate de startup em `src/executionGates.ts` / `useExecutionFlow.ts` aplicado à toolbar inteira.
- Shot: `A-21-multi-open.png`.

### Médio

**5. [I] A escolha do startup project não persiste** ao reabrir a solução (2×); todo dia começa com o passo extra do item 4.

**6. [I] "Build" compila só o startup project; o botão diz "Rebuild"** — erro em projeto não referenciado (inclusive o de testes) só aparece ao rodar testes. O comando é `dotnet build …/Hello.csproj -c Debug --no-restore`; não há Build Solution. 

**7. [I] Erro de compilação do projeto de testes na descoberta fica só no log cru** — "Discovery failed · exit 1", Problems com 0 linhas, texto não clicável, Run all/selected desabilitados. O erro de build normal é clicável. Shot: `A-40-tests-compile-error.png`.

**8. [I] Crash por exceção não tratada aparece como "Run exited · exit unknown"** em estilo neutro, sem destaque nem código/sinal (.NET aborta com SIGABRT no macOS); a linha `at Program.<Main>$ in …:line 3` não é clicável. Causa provável: mapeamento de saída por sinal em `crates/host` e log `<pre>` sem parser de stack trace. Shot: `A-23-crash-run.png`.

**9. [I] Reabrir o workspace não restaura abas, cursor nem breakpoints** (2×; Save all/Discard em si corretos). O SPEC só promete configurações por solução, então não é regressão. Shot: `A-31-reopen.png`.

**10. [I] Debug de teste desabilitado de forma permanente** ("not part of this version (P1)"). Lacuna conhecida; o roteiro "depurar um teste" não é possível.

**11. [I] Debug: clicar num frame abre uma aba `Program.cs` duplicada a cada clique** — caminho absoluto do frame vs. relativo da aba (`DebugPanel.tsx onOpenFrame` → `WorkspaceView.navigateTo`).

**12. [I] Debug: sem avaliação por hover no editor** (2,5 s sobre `person`: nada; sem `hoverTooltip` em `CodeEditor.tsx`). Só Evaluate/Watch.

**13. [I] Duplo clique em arquivo fechado no Solution tree abre duas abas idênticas** (ambas `*`; "2 modified buffers" para 1 arquivo). Clique simples abre uma. 2×. Causa provável: handlers click + dblclick sem dedupe.

**14. [I] Diálogos Push/Update abertos por atalho não tomam foco**: a digitação cai no buffer oculto e Esc não fecha (2×). Causa provável: `src/git.tsx` ~528–590 sem autoFocus/trap/Escape.

**15. [B/I] Back/Forward sempre desabilitados; `⌘[` e `⌘⌥←/→` não fazem nada** (após Go to Definition não há histórico). Fix ligado ao #16/#19 da rodada anterior.

**16. [I] Go to Line (`⌘G`) é stub** ("arrives in F3"); no editor cai no Find-next do CodeMirror; `⌘L` (Rider) não faz nada.

**17. [I] Ações ausentes (conjunto)**: fechar aba (`⌘W`), próxima/anterior aba, Recent Files (`⌘E`), Go to File no estilo Rider (`⌘⇧O`), Find in Files (`⌘⇧F`, ver rodada anterior), mostrar/ocultar painéis (`⌘1`/`⌥1`), Format (`⌘⌥L`) e format on save, Organize usings (`⌘⌥O`), Go to Implementation, split, buscar/substituir no projeto, logpoint e desabilitar breakpoint individual. Find Action não encontra format, organize, using, tab, recent, duplicate, back, forward, split, replace, implementation. (Os dois últimos itens de debug estão documentados como limitação em `decisions.md`.)

**18. [I] Divergências do perfil "Rider (macOS)"**: `⌘D` seleciona próxima ocorrência (Rider: duplicar linha); `⌘⌫` junta linhas (Rider: apagar linha); `⌥↑/↓` move e `⌥⇧↑/↓` copia (estilo VS Code); `⌘T` = Search Everywhere (Rider: `⇧⇧`/`⌘⇧O`); `⌘Y` nada.

**19. [I] Painel inferior padrão (220 px) mostra ~1 item de Problems**; cabeçalho contraditório ("0 errors · 0 warnings" ao lado de "Build · 2 problems" e "2 errors" na status bar) e aviso de "no analysis session published diagnostics" mesmo com o erro de build listado. Shot: `A-06-problems-big.png` (painel ampliado).

### Baixo

20. Run configurations: nomes duplicados aceitos sem aviso (inclusive o da default); cwd inexistente/`../../..` e `.env` com `../../../../etc/hosts` aceitos; salvar com cwd/.env inválido grava ("saved but cannot run") enquanto nome vazio diz "nothing was saved".
21. Arquivo alterado no disco: buffer limpo não recarrega; só detecta ao receber foco e o banner diz "discard my edits" mesmo sem edições. Arquivo apagado: erro correto, mas árvore e aba não indicam a remoção. 
22. Status bar mostra "Keymap: IntelliJ / Rider" fixo (`src/shell/StatusBar.tsx:93`) com perfil ativo "Rider (macOS)".
23. Debug: nomes longos de frame vazam sobre Locals ; árvore de Locals/Watches sem indentação e expansão perdida a cada step; Dictionary mostra internals (`_buckets`, `_entries`); `Person` mostra `EqualityContract` primeiro; `Compute(1,2)` avaliado dá `error: 0x80070057`; frames async aparecem como `<<Main>$>d__0.MoveNext()`.
24. Debug: após Pause o frame 0 é `Thread.Sleep()` sem fonte; o primeiro frame com código do usuário não é selecionado (Locals vazio, sem linha destacada). Painel de Debug empilha tudo na vertical.
25. Teclado: passos de debug (F7/F8/F9/⇧F8) e `⇧F5` sem sessão ficam silenciosos enquanto `⌥F9`/`⌘F2` mostram erro; Search Everywhere reabre com a consulta anterior e a digitação se anexa (rodada anterior, reconfirmado); code actions mostram "no kind" em todas as linhas; "Generate class in new file" mostra erro de path inline; Keymap agrupa Build/Run/Debug em "—"; `Ctrl+Space` conflita com troca de input source do macOS.
26. `Alt+F5` instalou netcoredbg e ligou a flag experimental sem plano de confirmação (o tooltip diz que faz; verificar se é intencional frente a `decisions.md` ticket 13).

## Tabela de atalhos (esperado × real)

| Atalho | Esperado | Real |
|---|---|---|
| ⌘B / F12, ⇧F6, ⌥Enter, Ctrl+Space | Definition, Rename, Code actions, Completion | OK |
| ⌘F, ⌘S, ⌘/ , ⌘Z, ⇧⌘Z | Find no arquivo, salvar, comentar, undo, redo | OK |
| ⌘⇧T / ⌘T | Go to File / Search Everywhere | OK (diverge do Rider) |
| ⌘F9, ⌃F5, ⌥F5, ⌃R, ⇧F5 | Build, Run, Debug, Run, Stop | OK |
| F7, F8, F9, ⇧F8, ⌥F9, ⌘F2, ⌘F8 | Into, Over, Resume, Out, Run to Cursor, Stop, Toggle BP | OK |
| ⌘U + L/F/D | Testes | OK fora do editor; **quebrado no editor** (#2) |
| ⇧⌘K, ⌃D | Push, Debug | **engolidos** (#3) |
| ⌘G, ⌘L | Go to Line | stub / nada (#16) |
| ⌘⇧F | Find in Files | nada (rodada anterior) |
| ⌘W, ⌘⇧], ⌃Tab, ⌘[, ⌘⌥←/→, ⌘E, ⌘⇧O, ⌘1/⌥1, ⌥F12, ⌘⌥L, ⌘⌥O | abas, back/forward, recent, painéis, format, usings | não existem (#15, #17) |
| ⌘D, ⌘⌫, ⌘Y | duplicar linha, apagar linha, redo | divergem / nada (#18) |

## O que funcionou bem

- Editar → ⌘S → ⌘F9 com erros: 2 erros (CS0029 5:9, CS0103 5:14) com sublinhado e marcador, clique em Problems leva a `Ln 5, Col 9/14`; corrigir → Build succeeded → `⌃R` imprime a saída com cwd e args corretos.
- Build/Run com buffer sujo: diálogo claro (Save all and build / Build saved / Cancel); edit + undo limpa a flag (achado 3 anterior corrigido). Run já em curso: botão desabilitado com motivo.
- Run configurations: duplicar, editar nome/args/cwd/env/.env, ativar; Build + Run usam args, env e cwd configurados, stderr marcado, persistem após recarregar.
- Testes: descobrir (4), Run all (2 passed/1 failed/1 skipped), selected, Rerun failed, detalhes com stack e "Open source position" na linha 29; Quick List explica cada item desabilitado.
- Debug (netcoredbg 3.2.0): sem o debugger, `experimental_required`/`tool_missing` com mensagem clara e instalação em ~2 s pela UI; breakpoint de linha, condicional (`i == 3`), adicionar/remover com sessão ativa, mute; Step Into/Over/Out, Continue, Run to Cursor, Pause, Stop (processos somem), Restart (novo pid, breakpoints verificados); Locals/Watches/Evaluate corretos, objetos e coleções expandem, erros de avaliação claros (`foo +` → CS1733); exceção não tratada para na linha do `throw`; breakpoints em async (inclusive após `await`).
- Edição: completion (Ctrl+Space, 200 itens), Find Usages (4 refs), quick fix com preview/Apply/undo, Rename com preview, diagnósticos ao digitar (~1 s), Go to Definition, buscar/substituir no arquivo, instalação do csharp-ls pelo app.
- Robustez: fechar com buffer sujo (Save all/Keep editing/Discard gravam corretamente); conflito de disco bloqueia Save; processo que ignora SIGTERM recebe SIGKILL após a graça (3,1 s); Stop em loop longo dá exit 143 sem sobras no `ps`.
- Recursos (sessão de debug ~11 min, ~280 ações, amostras a cada 10 s):

| Processo | CPU média | CPU máx | RSS início → fim |
|---|---|---|---|
| host-http | 1,9 % | 5,8 % | 12,3 → 11,4 MB |
| netcoredbg | 0,14 % | 2,5 % | 57,8 → 19,0 MB |
| debuggee (dotnet) | 0,12 % | 1,3 % | 37,0 → 13,8 MB |

  Após Stop, netcoredbg e dotnet somem na hora e o host fica a 0 % / 11,3 MB por mais de 1 min.

## O que não foi testado (e por quê) — tudo "não verificado"

- **App Tauri real**: acessibilidade e gravação de tela negadas ao `agent-desktop`; não dá para ver nem dirigir a janela. Atalhos de sistema/navegador (`⌘W`, `⌘T`, `⌘F`) só foram vistos no Chrome. CPU/memória do Tauri e do csharp-ls não medidos.
- Debug de teste (desabilitado no produto) e aba Tests → "Debug selected".
- Restart depois de editar o fonte (rebuild implícito), "Break on thrown exceptions", painel Threads com programa multi-thread, aba Console do debug, falha de instalação do netcoredbg na UI (só via host).
- Cancelar build/test em andamento (o build termina antes do clique).
- Signature help, Go to Symbol e Find in Path remapeado (interrompidos); `Ctrl+F5`; teclas F sem Fn.
- Windows e macOS Intel (alvo verificado é macOS ARM64).
- Observação não confirmada: a primeira tecla (F8/F9) logo após uma pausa pareceu ignorada com espera de 0 ms; com ≥300 ms funcionou (possível corrida do script).

## Limitações desta entrega

- Um erro meu na limpeza apagou as screenshots originais. Foram recapturadas: `A-06-problems-big`, `A-19-chord-L`, `A-21-multi-open`, `A-23-crash-run`, `A-31-reopen`, `A-40-tests-compile-error`, `C-06-cmdshiftk`. Os achados sem imagem (debug B-*, A-11, C-25) continuam válidos — foram reproduzidos ≥2× pelos agentes —, mas a recaptura não foi concluída.
- Na estimativa de CPU/memória, "Debug de ~11 min" é o que se mediu; não houve sessão de horas.
