# E2E exploratório: OpenRidr 0.3.0-alpha.5

- Data: 2026-11-03 · macOS 27.2 arm64 · branch `test/e2e-alpha5` (a partir de `chore/verify-app-real`, HEAD `5c83aae`)
- Alvo: cópia do DMG `OpenRidr_0.3.0-alpha.5_aarch64.dmg` em `~/Applications-test/OpenRidr-alpha5/OpenRidr.app`.
  `/Applications/OpenRidr.app` (alpha.4) não foi tocado.
- Ferramenta: agent-desktop 2.5 (superfície C), mais `doctor.sh` e uma exceção web descrita na seção 7.
- Nenhuma correção de código foi feita. Nada foi commitado além deste relatório, do log, das capturas e da skill nova.

## 1. Resumo

Total: **20 achados**. P0: 1 · P1: 1 · P2: 11 · P3: 7. Destes, 6 são da skill `verify-openridr` (BUG-009 a BUG-013 e BUG-020).

Riscos principais:

1. **BUG-001 (P0)**: Cmd+Q (menu ou AppleScript `quit`) com alterações não salvas encerra o app **sem perguntar** e perde a edição. Reproduzido 3 vezes. O botão Close da janela pergunta corretamente, então o defeito está no caminho de quit.
2. **BUG-002 (P1)**: Depois de apagar um `.cs` por fora, Debug fica bloqueado com a mensagem "Rebuild before debugging" mesmo após build verde. Só "Re-evaluate" libera.
3. **Integridade de arquivo**: save de CRLF vira LF (BUG-005) e buffer de aba limpa fica desatualizado sem aviso (BUG-003).

O que funcionou bem: Trust (persistência e revogação), diálogos de conflito com disco, Save/Cmd+S, Undo, Go to definition, diagnósticos LSP, Build/Run/Test (2 passed, 1 failed proposital, 1 skipped), mensagens de erro para caminhos inválidos, pasta vazia e duas soluções, nomes com espaço/acento/emoji, sem crash reports e com uso de memória baixo em idle.

Assinatura: validada (seção 2).

## 2. Validação da cópia

| Verificação | Resultado |
|---|---|
| `Info.plist` | `CFBundleShortVersionString` = **0.3.0-alpha.5**, `CFBundleVersion` = 0.3.0-alpha.5, `CFBundleIdentifier` = com.openridr.desktop |
| `codesign -dv --verbose=4` | Authority **Developer ID Application: <nome> (Team K6M85697D4)**, TeamIdentifier **K6M85697D4**, flags `runtime`, timestamp 3 Nov 2026 16:41:34 |
| `codesign --verify --deep --strict` | valid on disk; satisfies its Designated Requirement (exit 0) |
| `spctl -a -vv` | **accepted**, source=**Notarized Developer ID** (exit 0) |
| `xcrun stapler validate` | "The validate action worked!" (exit 0) |
| Quarentena na cópia | nenhum `com.apple.quarantine` (ditto preservou assinatura) |
| `/Applications/OpenRidr.app` | segue **0.3.0-alpha.4**, não tocado |

## 3. Tabela de bugs (ordenada por prioridade)

Legenda: P0 perda de dados/crash · P1 feature quebrada · P2 incômodo funcional/integridade · P3 polimento/doc/lacuna.
Ambiente de todos: cópia alpha.5, `OPENRIDR_HOME=/tmp/alpha5-desktop/home`, solução `OrderDesk` copiada para `/tmp/alpha5-desktop/work`.
Evidência textual de todos: `notes-log.md` (linha correspondente).

| ID | P | Título | Evidência |
|---|---|---|---|
| BUG-001 | **P0** | Quit (Cmd+Q / menu Quit / AppleScript) com buffer sujo sai sem diálogo e perde a edição | notes-log; contraste em `screenshots/close-with-unsaved-dialog.png` |
| BUG-002 | P1 | Debug bloqueado após apagar `.cs` por fora, mesmo após rebuild; só Re-evaluate destrava | notes-log |
| BUG-003 | P2 | Buffer de aba limpa não recarrega quando o arquivo muda no disco (fica stale com a aba ativa) | notes-log |
| BUG-004 | P2 | Árvore não reflete criar/renomear/apagar externo até Re-evaluate | notes-log |
| BUG-005 | P2 | Save de arquivo CRLF converte todas as linhas para LF | notes-log |
| BUG-006 | P2 | Arquivo `.cs` em UTF-16 (com BOM) não abre | notes-log |
| BUG-007 | P2 | Argumento `.cs`/`.txt` é ignorado em silêncio (Welcome sem explicação) | `screenshots/paths-c7-only-cs-raw.png` |
| BUG-008 | P2 | Clicar Debug baixa e executa netcoredbg sem pergunta; o plano de Install pede confirmação explícita | notes-log |
| BUG-009 | P2 | **Skill**: `desktop.sh down` imprime "stopped" com a instância ainda viva | notes-log |
| BUG-010 | P2 | **Skill**: estados fixos em `/tmp/verify-openridr*` (rm -rf no `up`) geram colisão entre sessões | notes-log |
| BUG-011 | P2 | **Skill**: `doctor.sh` e `desktop.sh` apontam para `/Applications` fixo; não validam a cópia sob teste | seção 6 |
| BUG-012 | P2 | **Skill**: receitas da superfície C omitem que `type`/`press` precisam de `--headed` | notes-log |
| BUG-013 | P2 | **Skill**: docs descrevem diálogos que não existem (consent de debug, "diálogo de troca") e status defasado | seção 6 |
| BUG-014 | P3 | Executar o binário direto com o app aberto abre 2ª instância (sem repasse à existente) | notes-log |
| BUG-015 | P3 | Plano do netcoredbg diz "blocked on this platform" com o binário já instalado e Debug executado | notes-log |
| BUG-016 | P3 | `.sln` inválido: após Trust a mensagem diz "needs a resolved project SDK" | `screenshots/paths-c6-broken-sln-trust-after-trust.png` |
| BUG-017 | P3 | TFM inválido na 1ª avaliação: "Values shown are from the previous evaluation" e "stale · revision 1" | `screenshots/paths-c8-broken-tfm-trust-after-trust.png` |
| BUG-018 | P3 | Rótulos de acessibilidade ambíguos: "Run" 3× e "Save" repetidos no keymap; menu de projeto chama-se "Solution context menu" | notes-log |
| BUG-019 | P3 | Lacuna: árvore não tem criar/renomear/apagar arquivo (só alterações externas) | notes-log |
| BUG-020 | P3 | **Skill**: `doctor.sh` imprime fragmento de JSON na linha do agent-desktop | seção 6 |

Passos exatos de reprodução:

**BUG-001 (P0).** Abrir o `.sln` e confiar. Clicar em `Order.cs`. Digitar `QZ9` no editor (com `agent-desktop type --headed`). Confirmar aba "Order.cs, modified". Executar Cmd+Q (`agent-desktop press cmd+q --app OpenRidr --headed --force`), ou o menu OpenRidr > Quit, ou `osascript -e 'tell application "<bundle>" to quit'`.
- Esperado: diálogo "Close OpenRidr with unsaved changes?" (como o botão Close da janela) ou cancelamento.
- Obtido: o processo encerra sem diálogo. `grep QZ9 Order.cs` = 0 e reabrir o recente não restaura o texto.
- Reproduzido 3×: `QQ` (Cmd+Q via menu), `RR1` (AppleScript quit), `QZ9` (Cmd+Q com `--window-id`).
- Contraste: botão Close da janela com `CL7` e `DLG` mostra "Close OpenRidr with unsaved changes?" com Save all / Keep editing / Discard and close app. Save all gravou `CL7` no disco.

**BUG-002 (P1).** Abrir, confiar, Build (verde). Apagar `src/OrderDesk.Core/Order.cs` por fora. Build (verde). Aba Debug → Start debugging.
- Esperado: a sessão inicia, ou a mensagem é verdadeira e acionável.
- Obtido: "Debug was not started: 1 saved source changed after this build (src/OrderDesk.Core/Renamed.cs). Rebuild before debugging." (cita arquivo inexistente). A árvore ainda lista o arquivo. Re-evaluate + Build → mensagem some e Debug roda (terminou normalmente).

**BUG-003 (P2).** Aba `Program.cs` limpa e ativa. Por fora, prefixar uma linha no arquivo. Observar o editor por 20 s.
- Obtido: texto antigo, sem aviso. Recarrega só ao trocar de aba ou focar. Se o usuário digitar e salvar depois do foco, não há conflito (o foco já recarregou). Risco: código visível diferente do que compila.

**BUG-004 (P2).** Abrir sln. Criar `Extra.cs`, renomear `Order.cs` → `Renamed.cs`, apagar `OrderTests.cs`, tudo por fora. Esperar 20 s.
- Obtido: árvore idêntica. Re-evaluate mostra `Extra.cs` e `Renamed.cs` e remove `OrderTests.cs`.

**BUG-005 (P2).** Arquivo `Crlf.cs` com `\r\n`. Abrir, digitar `Q` no início, Save. `grep -c $'\r'` = 0 (3 linhas, `file` = ASCII text).
- Esperado: CRLF preservado. Obtido: LF em todas as linhas.

**BUG-006 (P2).** `.cs` com BOM UTF-16 (`iconv -t UTF-16`). Abrir pela árvore.
- Obtido: "Could not open … Could not read … as UTF-8 text". Mensagem clara, mas não há suporte nem opção de encoding.

**BUG-007 (P2).** Abrir o app com argv `A.cs` ou `notes.txt` (ou `scripts/openridr A.cs`).
- Obtido: Welcome sem mensagem. Esperado: explicar que é preciso `.sln`, `.slnx`, `.csproj` ou pasta. (A tela de pasta vazia já mostra mensagem assim.)
- **Correção do relatório (2026-11-04):** não reproduz no HEAD. O repro usou `scripts/openridr` sem `OPENRIDR_APP`, que abre `/Applications/OpenRidr.app` (0.3.0-alpha.4, anterior ao argv/open-with); as quatro capturas do achado são md5-idênticas e mostram a mensagem correta. O conserto desta rodada é no launcher (recusa extensão não suportada e imprime o bundle) + e2e; o status desta linha será finalizado depois.

**BUG-008 (P2).** Solução confiada, build atual. Clicar Debug (toolbar, sem menu). Sem nenhuma pergunta, o app baixa `netcoredbg-osx-arm64.zip` (3.2.0-1092) para `home/tools/netcoredbg` e inicia a sessão. Código: `useExecutionFlow.ts:291`, "One-click Debug: there is no consent question". Em contraste, "Install analysis" exige "Review install plan" e "Confirm and install".
- Decisão a tomar: ou o clique em Debug vale como consentimento (documentar), ou o download precisa de confirmação como o plano. O SHA-256 publicado e o limite de 20 MiB estão corretos no plano.

**BUG-009 (skill).** `desktop.sh up <bundle>` (PID A). Subir outra instância à mão (PID B). `desktop.sh down` → "stopped". `pgrep -x openridr` continua com B. Causa: `app.pid` guardava um PID antigo (36017).

**BUG-010 (skill).** `desktop.sh up` faz `rm -rf $state` no caminho fixo `/tmp/verify-openridr-desktop`, e `web.sh up` faz o mesmo com `/tmp/verify-openridr`. Durante esta sessão outra sessão usava o estado `/tmp/verify-openridr-desktop` com o app ativo (PID 13565). Usei estados `/tmp/alpha5-*` para não colidir. O `up` recusa se houver `openridr` rodando, mas não protege a pasta de outra sessão parada.

**BUG-011 (skill).** `doctor.sh` lê só `/Applications/OpenRidr.app` e avisa "rebuild before trusting desktop results" mesmo quando o alvo é outra cópia. `desktop.sh up` usa `/Applications` como padrão. Não há como dizer ao doctor qual bundle validar.

**BUG-012 (skill).** `agent-desktop type @ref "x"` no `Editor` sem `--headed`: `ACTION_FAILED … post_action_verification` (`delivered_unverified`), texto não entra. `agent-desktop press cmd+s --app OpenRidr` sem `--headed`: "Application has no verified focused element for keyboard delivery". Com `--headed` (depois de `focus-window`) funcionou. A skill manda "prefira ações headless" e não avisa.

**BUG-013 (skill).** Ver seção 6, itens de drift.

**BUG-014 (P3).** Executar o binário direto (não via LaunchServices) com o app já aberto: sobe segunda instância com o mesmo `OPENRIDR_HOME`. Via `open -a` (que `scripts/openridr` usa) houve uma instância só e a decisão de Trust foi reutilizada. Só afeta quem executa o binário direto.

**BUG-015 (P3).** Aba Debug mostra "Apple Silicon debugger: experimental · disabled" e o plano "stays blocked on this platform", enquanto `tools/netcoredbg/3.2.0-1092` existe e a sessão rodou. Pode ser flag de sessão, mas a UI não explica.

**BUG-016 (P3).** `Broken.sln` (texto livre): após Trust, "Evaluation needs a resolved project SDK. Host SDK 10 is never substituted." O problema é o arquivo, não o SDK.

**BUG-017 (P3).** `OrderDesk.csproj` com `net99.0`: na primeira avaliação aparece "Evaluated with errors · stale · revision 1" e "Values shown are from the previous evaluation". Não existe avaliação anterior.

**BUG-018 (P3).** Keymap lista "Run" três vezes e "Save" duas vezes (ações distintas com o mesmo rótulo). O menu de contexto de projeto se chama "Solution context menu". A árvore principal tem 54 botões, todos com nome; o problema é de desambiguação.

**BUG-019 (P3, lacuna).** Árvore sem criar, renomear ou apagar arquivo. Os menus de projeto e de Options só têm ações de projeto e avaliação.

**BUG-020 (P3, skill).** A linha do agent-desktop no `doctor.sh` imprime `note  agent-desktop "accessibility":{"state":"granted"`: o `grep -o` deixa o fragmento de JSON cru.

## 4. Matriz de cenários

Legenda: PASS · FAIL (bug) · PARCIAL · N/T (não testado) · N/A.

| Área | Cenário | Resultado | Nota |
|---|---|---|---|
| Abertura | `.sln` por argv → diálogo Trust | PASS | `d01-launch-argv-trust.png` |
| Abertura | `.slnx` por argv | PASS | Trust prompt com nome correto |
| Abertura | `.csproj` isolado | PASS | Trust prompt (`kind` project) |
| Abertura | Pasta vazia | PASS | "No .sln, .slnx or .csproj found in …" |
| Abertura | Pasta com 2 soluções | PASS | "Choose a workspace"; não escolhe sozinho |
| Abertura | Caminho inexistente | PASS | "Workspace path not found: …" |
| Abertura | Caminho com espaço, acento e emoji | PASS | `paths-c1-espaco-acento-emoji.png` |
| Abertura | `.cs` / `.txt` como argumento | FAIL | BUG-007 |
| Abertura | `.sln` inválido | PARCIAL | Abre com Trust; mensagem imprecisa (BUG-016) |
| Abertura | TFM inválido | PARCIAL | Erro exibido; "stale" enganoso (BUG-017) |
| Abertura | Solução grande (dezenas de projetos) | N/T | não gerada |
| Abertura | Recentes na tela inicial | PASS | "20 minutes ago · Trusted"; clique reabre e restaura a aba ativa |
| Abertura | Reabrir sozinho o último workspace | N/A | App lista em Recentes e não reabre sozinho (comportamento, não bug) |
| Abertura | Abrir outro caminho com workspace aberto | PASS | Trust direto (sem diálogo de troca, ver BUG-013); Cancel mantém o anterior |
| Abertura | CLI `scripts/openridr` (`open -a`) | PASS | Instância única; decisão reutilizada sem prompt |
| Abertura | Binário direto com app aberto | FAIL | BUG-014 |
| Abertura | Finder "Abrir com" e arrastar para o Dock | N/T | sem acesso ao Finder/Dock pelo agent-desktop nesta sessão |
| Trust | Trust project → `trusted.json` + "Trusted mode" | PASS | `{"…/OrderDesk.sln":"trusted"}` |
| Trust | Persistência após relaunch | PASS | Decisão restaurada, sem prompt |
| Trust | Revoke trust | PASS | grava `"restricted"`; mensagem "Read from project text only" |
| Trust | Cancel no prompt não grava | PASS | Verificado com `.csproj` |
| Trust | Restricted: Build/Run/Debug bloqueados com motivo | PASS | "Trust the workspace before debugging; nothing runs in Restricted mode." |
| Árvore | Abrir arquivo pela árvore | PASS | |
| Árvore | Expandir/colapsar tudo | N/T | |
| Árvore | Mudanças externas (criar/renomear/apagar) | FAIL | BUG-004 (só após Re-evaluate) |
| Árvore | Criar/renomear/apagar pela UI | N/A | não existe (BUG-019) |
| Árvore | Menu de contexto de projeto | PARCIAL | ações de projeto; rótulo (BUG-018) |
| Editor | Várias abas e troca | PASS | Program.cs, Order.cs, Extra.cs |
| Editor | Dirty marker e "1 modified buffer" | PASS | |
| Editor | Cmd+S grava no disco | PASS | |
| Editor | Botão Save grava no disco | PASS | |
| Editor | Fechar aba sujo → diálogo; Esc mantém; Discard descarta | PASS | "Close modified file?" (Save / Discard and close) |
| Editor | Undo (Cmd+Z) | PASS | reverte a digitação e a aba volta a limpa |
| Editor | Redo | N/T | |
| Editor | Busca (Cmd+F) | N/T | `press cmd+f` deu erro de tool (APP_UNRESPONSIVE) |
| Editor | Conflito de disco com buffer sujo | PASS | "changed on disk … Reload from disk / Overwrite disk"; disco preservado |
| Editor | Buffer limpo com disco mudado | FAIL | BUG-003 |
| Editor | Arquivo apagado por fora com aba aberta, depois Save | PASS | "Save failed; buffer kept. File not found" (não recria) |
| Editor | CRLF: abre | PASS | |
| Editor | CRLF: salvar preserva CRLF | FAIL | BUG-005 |
| Editor | UTF-16 com BOM | FAIL | BUG-006 |
| Editor | Binário com `.cs` | PASS | erro claro |
| Editor | Arquivo 28 MB | PASS | abre; tempo não medido; RSS ~180 MB |
| LSP | Go to definition (Cmd+B) | PASS | abre `Order.cs` em `Ln 3, Col 21` |
| LSP | Diagnósticos (csharp-ls) | PASS | `CS0103`, `CS0841`, `CS1001` com origem "LSP · csharp-ls" |
| LSP | Completion (Ctrl+Space) | PARCIAL | "csharp-ls returned no items" com código quebrado; não validado com código válido |
| LSP | Hover | N/T | exige mouse (`--headed`), não feito |
| LSP | Rename / code actions | N/T | |
| Build | Build com sucesso | PASS | "Build succeeded · exit 0"; DLL gerada |
| Build | Build com erro → Problems navegáveis | PASS | CS1003/CS1002 em `Program.cs` 3:17 e 3:18; clique posiciona `Ln 3, Col 18` |
| Run | Run | PASS | `Order A-104: R$ 42,00`, exit 0 (moeda do locale) |
| Tests | Discover | PASS | 4 testes |
| Tests | Run all | PASS | 2 passed, 1 failed (`Deliberate_failure_for_runner_evidence`, proposital), 1 skipped |
| Tests | Run selected / Debug selected / Rerun failed | N/T | |
| Debug | Start após mudança de fonte | FAIL | BUG-002 |
| Debug | Consentimento | FAIL (decisão) | BUG-008: sem pergunta |
| Debug | Sessão, breakpoints, passos | PARCIAL | sessão rodou e terminou (console). Breakpoints N/T |
| Analysis | "Install analysis" → plano → confirmar | PARCIAL | fluxo com confirmação explícita; o clique de confirmação caiu no plano do netcoredbg |
| Analysis | Falha de rede simulada | N/T | `HTTPS_PROXY` inválido configurado, instalação não concluída |
| Janela | Fechar janela com buffer sujo → diálogo | PASS | Save all / Keep editing / Discard and close app |
| Janela | Cmd+Q / quit com buffer sujo | FAIL | BUG-001 (P0) |
| Janela | Redimensionar 760×560 | PARCIAL | captura em `resize-760x560.png`; tool deu timeout parcial; layout não analisado |
| Janela | Múltiplas janelas (Cmd+N) | N/T | |
| Tema | Claro/escuro | N/A | Settings mostra "Light theme is unavailable in this build" |
| Acessib. | Labels dos botões (árvore) | PASS | 54 botões, 0 sem nome |
| Acessib. | Foco por teclado (Tab) | N/T | foco dentro do WebView não aparece na árvore AX (só `webarea`) |
| Atalhos | Keymap "Rider (macOS)" | PASS (listagem) | Listados: ⌘S, ⌘W (Close Tab), ⌘B, ⇧⌘A, ⌃R, ⌃D, ⌘, (Settings); só ⌘S, ⌘B e ⌘Z foram executados |
| Atalhos | Colisão com macOS | PARCIAL | ⌘W e ⌘, listados, não executados; ⌘Q funciona mas perde dados (BUG-001) |
| Recursos | Idle (1 min, Order.cs aberto) | PASS | RSS ~96 MB, CPU 0,3% |
| Recursos | Crash reports | PASS | 0 em `~/Library/Logs/DiagnosticReports` para openridr |
| Recursos | Log do app | PARCIAL | stdout vazio; sem log útil no processo |
| Robustez | `kill -9` durante build | PARCIAL | build terminou antes do kill; sem órfãos (dotnet, BuildHost, csharp-ls) com caminho do teste |
| Robustez | Órfãos de runs anteriores | PASS | 53 `csharp-ls`/BuildHost órfãos encerrados antes do início |
| Web (exceção) | `web.sh up`, abrir `.sln`, Trust, árvore, abrir `Program.cs` | PASS | `trusted.json` gravado; host `app.state` coerente; parado após a regra do usuário |
| Playwright | `tests/e2e/*.spec.ts` | N/T | excluído pela regra "sem web"; só checado se os caminhos existem |
| Formal | `cargo test -p openridr-host --test trust_model` | N/T | código de trust não mudou nesta sessão |

## 5. Não verificado, e por quê

- **Falha de rede** (instalar analysis com proxy inválido): o clique de confirmação caiu no plano do netcoredbg. Não concluído.
- **Hover, rename, code actions, completion com código válido**: exigem mouse ou código válido, não feitos.
- **Breakpoints e passos do debugger**: a sessão rodou e terminou sem breakpoint.
- **Foco por teclado e Tab**: o foco dentro do WebView não aparece na árvore de acessibilidade.
- **Finder "Abrir com", arrastar para o Dock, múltiplas janelas (Cmd+N)**: não executados.
- **Solução grande, expandir/colapsar, Redo, busca Cmd+F**: não executados ou inconclusivos (erro da tool).
- **Redimensionar**: captura feita, layout não analisado.
- **Windows e Intel**: fora de escopo (ver AGENTS.md).

## 6. Achados sobre a skill `verify-openridr`

A pergunta foi: funciona de ponta a ponta, e a superfície C cobre o uso do agent-desktop?

Funcionou:
- `doctor.sh` executa e sai com 0.
- `web.sh up/status/down` sobe UI (1520) e host (4417), e derruba limpo. Porta livre depois.
- `desktop.sh up <bundle>` aceita um bundle alternativo e passa o `.sln` como argv. Abre no Trust prompt.
- `features/trust.md` (B): `trusted.json`, `app.state` (`workspace.mode`, `trust.decision`, `trust.granted`) e `status-mode` batem com o que o app mostra.
- `features/solution-tree-editor.md`: três projetos com "Evaluated", abrir arquivo, Program.cs com o conteúdo do disco.

Não funcionou ou é enganoso:
- **BUG-009**: `desktop.sh down` diz "stopped" com a instância viva. Ver item 3.
- **BUG-010**: caminhos fixos com `rm -rf`. Ver item 3.
- **BUG-011**: `doctor` e `desktop` fixos em `/Applications`. Não dá para validar a cópia. Esta sessão só usou a cópia porque passou o caminho explicitamente.
- **BUG-012**: receitas de teclado e digitação quebradas sem `--headed`. Ver `test-app-agent-desktop`.
- **BUG-013 (drift)**:
  - `features/build-test-debug.md` cita `debug-consent-dialog`. O código diz "there is no consent question" (`useExecutionFlow.ts:291`). Não há diálogo.
  - `features/open-workspace.md` e `features/open-with.md` citam "diálogo de troca (externalOpen)". Em `WorkspaceView.tsx:3816` o caminho novo vai direto para `openSolution`.
  - `features/open-with.md` diz "C não pôde ser provada (agent-desktop PERM_DENIED)". Hoje a permissão está concedida e o teste foi feito.
- **BUG-020**: linha de accessibility do doctor com fragmento de JSON.

Lacunas, que não foram bugs: o `desktop.sh down` não sabe qual PID é o real; a skill não diz quando usar `--headed`; não tem receita para diálogo de unsaved changes, `--window-id` nem para `close-app` ambíguo.

Decisão: a skill `verify-openridr` cobre o caminho principal, mas falta a parte de receitas do agent-desktop. Criei a skill separada `test-app-agent-desktop` (`.agents/skills/test-app-agent-desktop/SKILL.md`) com as receitas testadas nesta sessão, inclusive as falhas. A `verify-openridr` ainda não aponta para ela; recomendo acrescentar uma linha quando os itens 1 a 3 forem corrigidos.

Não executados pela skill nesta sessão: Playwright (`bunx playwright test …`) e `cargo test -p openridr-host --test trust_model`. Verificado só que os arquivos referenciados existem.

## 7. Exceção web e regras

- A superfície web (`web.sh`, Vite 1520, host 4417, navegador) foi usada **uma vez**, para abrir o `.sln`, confiar, ver a árvore e abrir `Program.cs`. Depois a regra mudou para "teste pelo desktop", e a superfície web foi derrubada. Registrado em `notes-log.md`.
- O tab do navegador foi fechado (`browser.close`).
- `launchctl setenv OPENRIDR_HOME` foi usado para o caminho LaunchServices e depois desfeito (`launchctl getenv` vazio).

## 8. Decisões e suposições

- **Cópia**: `ditto` do DMG montado (`hdiutil attach -readonly`) para `~/Applications-test/OpenRidr-alpha5/`. O DMG foi desmontado no fim. A cópia ficou para reuso.
- **Processos**: encerrei 53 `csharp-ls`/BuildHost órfãos de runs anteriores (ppid 1, `openridr-e2e-home`). Não encerrei o zsh de sessão concorrente (PID 13704) nem o app 13565 de outra sessão. Esperei o 13565 sair antes da superfície C.
- **Worktree compartilhada**: troquei a branch da worktree `<repo>`, que outra sessão também usava (`verify-openridr-app`). Pedido explícito, mas vale saber.
- **`--headed`**: a skill diz que depende de permissão do usuário. Assumi que o pedido de testar atalhos e digitação cobre isso, e restringi a uso com o app em foco.
- **`press cmd+q --force`**: usado de propósito para testar o quit com buffer sujo. Em dados reais não use.
- **Dados de teste**: só `/tmp/alpha5-*`, `~/Applications-test/`, e `docs/evidence/e2e-alpha5/`. Fixtures do repo não foram alteradas. `fixtures/OrderDesk` já tinha `bin/` e `obj/` antes da sessão.
- **Sem correções**: nenhum bug foi corrigido. Só o relatório, o log, as capturas e a skill nova foram commitados localmente.

## 9. Entregáveis e contagem

- Este relatório: `docs/evidence/e2e-alpha5/REPORT.md`
- Log bruto: `docs/evidence/e2e-alpha5/notes-log.md`
- Capturas: `docs/evidence/e2e-alpha5/screenshots/` (Trust, paths, dialogs, resize, a11y, reopen)
- Skill nova: `.agents/skills/test-app-agent-desktop/SKILL.md`

Contagem: **P0: 1 · P1: 1 · P2: 11 · P3: 7** (total 20).

## 10. Status após as correções (branch `fix/e2e-alpha5`, 2026-11-04)

Spec: [`../SPEC-FIX-ALPHA5.md`](../SPEC-FIX-ALPHA5.md). Tickets: [`../../tickets-fix-alpha5/`](../../tickets-fix-alpha5/). Evidência: [`../fix-alpha5/VERIFICATION.md`](../fix-alpha5/VERIFICATION.md).

| ID | Status | Commit | Prova |
|---|---|---|---|
| BUG-001 | **corrigido** (Cmd+Q, menu Quit, botão Close) | `ceaa473` | diálogo no app real; Keep editing preserva; Save all grava; Close sem regressão |
| BUG-002 | **corrigido** | `f4885ad` | app real: delete externo + rebuild verde → Debug inicia |
| BUG-003 | **corrigido** | `d34a98e` | app real: texto externo aparece na aba ativa em ~3 s |
| BUG-004 | **corrigido** | `403e330` | app real: criar/renomear/apagar externo aparece em ~4 s |
| BUG-005 | **corrigido** | `e59bd4e` | app real: CRLF continua CRLF depois de editar e salvar |
| BUG-006 | **corrigido** | `e59bd4e` | app real: UTF-16LE com BOM abre, edita e salva no mesmo encoding |
| BUG-007 | **corrigido no script/relatório** | `4c0dda5` | não reproduzia no HEAD (alpha.4); script valida e imprime bundle; e2e cobre `.cs`/`.txt` |
| BUG-008 | **corrigido** (comportamento mantido) | `a5680e7`, `d0f56c1` | checksum, retry, progresso e registro de origem/versão/checksum; sem diálogo novo |
| BUG-009 | **corrigido** | `0ce59af` | `down` valida PID, espera e falha com instância restante |
| BUG-010 | **corrigido** | `0ce59af` | estado por usuário, impresso, rotação em vez de `rm -rf` |
| BUG-011 | **corrigido** | `0ce59af` | doctor aceita bundle e valida versão; desktop imprime versão e avisa divergência |
| BUG-012 | **corrigido** | `0ce59af` | docs exigem `--headed` + `focus-window` para `type`/`press` |
| BUG-013 | **corrigido** | `0ce59af` | docs sem `debug-consent-dialog`/diálogo de troca; status de PERM_DENIED atualizado |
| BUG-014 | **corrigido** | `f6cd2da` | app real: segunda execução direta sai e encaminha o `.sln`; um processo |
| BUG-015 | **corrigido** | `a5680e7` | app real: "experimental · enabled" com motivo; sem "blocked on this platform" |
| BUG-016 | **corrigido** | `4c0dda5` | `.sln` sem projetos culpa o arquivo, não o SDK |
| BUG-017 | **corrigido (texto)** | `4c0dda5` | `revision 1` não diz "previous evaluation"; stale real após restore continua |
| BUG-018 | **corrigido** | `24655a9` | nomes acessíveis distintos; menu de projeto "Project context menu" |
| BUG-019 | **corrigido (mínimo)** | `dfc0857`, `d0f56c1` | app real: criar/renomear/apagar com confirmação; nó de projeto protegido |
| BUG-020 | **corrigido** | `0ce59af` | doctor imprime `granted`/`denied`, sem JSON cru |
| Camada SDK | **implementada** | `9ab2b62`, `ff297da`, `3df4538` | app real: runtime 10.0.14 + SDK 8.0.408, sem pedir 10; descoberta ampla; csharp-ls com progresso/retry |
| netcoredbg embutido | **implementado** | `f804cd8` | app real com home vazia e DMG read-only; app 14,68→23,85 MiB; DMG 5.671.389→10.454.440 B; notarizado |

**Não corrigido / limitação declarada:**

- **Dock Quit e AppleScript `quit`**: usam `terminate:` nativo do macOS; o Tauri não expõe `applicationShouldTerminate`, então só Cmd+Q, menu Quit e o botão Close passam pelo diálogo. Registrado em `decisions.md`.
- **Capturas novas**: Screen Recording ficou negado nesta sessão; a prova da superfície C é snapshot de acessibilidade + estado em disco, sem PNG.
- **Progresso visual no app**: o download do netcoredbg (3.4 MB) conclui antes de um snapshot; o do csharp-ls não foi feito pela UI nesta sessão. Componente, erro e retry são cobertos por host/e2e.
- **Menu de arquivo** ainda se chama "Solution context menu" (o achado citava o menu de projeto, corrigido).
- **`desktop.sh up`** ainda usa `/Applications` por padrão, mas imprime a versão e avisa quando difere de `package.json`.
- **Embutir o netcoredbg** não foi implementado por decisão explícita; só a proposta medida.

Itens do relatório marcados N/T (hover, rename LSP, breakpoints, Finder "Abrir com", múltiplas janelas, redimensionamento analisado) continuam fora desta rodada e não são regressão.

Correções da revisão final: `d0f56c1` (caminho promovido do adaptador e proteção do nó de projeto) e `226a3aa` (erros dos listeners do shell e rechecagem antes do save).
