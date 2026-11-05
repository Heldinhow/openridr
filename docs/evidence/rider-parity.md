# Evidência — Paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Plataforma: macOS ARM64.
Bundle de debug do checkout (`0.3.0-alpha.6`), cópia isolada da fixture `OrderDesk`,
`csharp-ls` 0.28.0 instalado pelo próprio app no home isolado. Entrada física (`--headed`)
autorizada pelo Helder. Tickets 13–16 adiados, não verificados.

## Rodada final desta sessão (2026-11-04, depois das correções de foco e drawer)

- Playwright, specs 01–12 + `csharp-ls.spec.ts`: **128 passaram, 8 falharam, 1 pulado** na rodada
  completa. Das 8: `find-usages` "lists the real references" (existe no `main`), 2 de
  `run-configs` "rebuild fails" e "refuses a failed saved build" (conhecidas do HEAD), e 5
  instáveis sob carga (`run-configs` "saves and rebuilds", `build` "a real build reports",
  `debugger` "debugs a real build", `csharp-ls` "rename previews" e "D-12 focus"). Essas 5
  **passam isoladamente** (`-g`), então não são regressão comprovada; o rename passou sozinho também.
- Correções desta rodada, com prova no app real:
  - **Ticket 07 (foco após Enter):** `fix(navigation)` em `WorkspaceView.tsx`. Antes: Enter num
    arquivo novo deixava o foco em `webarea`, e ⌘F não abria o Find. Depois: foco em
    `textfield: Editor`, e ⌘F abre o Find (`find-open=2`). Causa: o editor remonta depois do
    `setTimeout(0)`, e o WKWebView demora mais para commitar. Correção: retentar o foco por frame
    **só enquanto o foco estiver em `document.body`** (não rouba de outro controle; uma primeira
    versão sem essa restrição quebrava o ⌘E, corrigida).
  - **Drawer de Workspace tools (ticket 11 de rodada anterior / item 2):** `fix(tools)`. Causa: o
    campo "Existing" vinha do plano gerado antes da instalação. Antes: "No launcher at the
    destination." com a análise `Ready`. Depois: "Verified launcher at the destination; confirming
    reuses it." Teste `csharp-ls.spec.ts` ("install plan is exact…"): vermelho antes (mostrou o
    texto antigo), verde depois.
  - **Ticket 11 via teclado:** caret na linha 12 (em branco) e ⌘F8 (`debug.toggleBreakpoint`):
    marcador vazado vermelho na linha 12 no app real (`11-breakpoint-linha-sem-codigo-teclado.png`).
    O texto "No code on this line…" está no `title` do marcador e não aparece na acessibilidade.
  - **Ticket 01 Restricted:** aviso "Trust the workspace before finding usages; no analysis runs in
    Restricted mode." e nenhum botão de instalar ou iniciar. Provado no app real.
  - **Filtro de `csharp-ls` (spec):** o cancel-inert contava processos de descoberta (`tests:`),
    que são automáticos desde o ticket 03. Corrigido no spec, não no produto.

## Playwright (superfície A), specs dos tickets 01–12 (rodada anterior)

- Rodada anterior: **116 passaram, 3 falharam, 1 pulado**.
- Falhas: `find-usages` "lists the real references" (lsp-state "Partial coverage", **existe
  no `main` `5c83aae`**, mesma asserção) e 2 de `run-configs.spec.ts` (falhas conhecidas do HEAD).
- Pulado: não identificado.
- Novos testes desta rodada: "Enter on a Search Everywhere file leaves the editor focused"
  (`focus-after-navigation`) e "Enter in the Find field moves to the next match…"
  (`editor-find-count`). Ambos passam no Chromium.

## Resultado por ticket (app real = `agent-desktop`)

| Ticket | Provado no app real | Não provado (motivo) |
|---|---|---|
| 01 análise | "Install analysis" abre plano sem download antes de confirmar; instalação pelo app; ⇧F12 com análise pronta lista 4 refs; "Start analysis" no aviso, e a consulta pendente roda sozinha ("4 references · session 2"); Rename (⇧F6) e code actions (⌥Enter) mostram "Start analysis" | Restricted mode no app: não rodado (exige reabrir o workspace em Restricted) |
| 02 keymap e janelas | Settings: perfil Rider, nota de chords fixos, ⌘T, ⇧F12, ⌘1…⌘8; ⌘T abre Search Everywhere; ⌘1 alterna Solution Explorer (1→0→1); ⌘8 alterna Tests (1→0) | Duplo Shift: não reproduzido pelo driver (`press`, `key-down`/`key-up` não disparam o gesto). Código lido em `WorkspaceView.tsx:3544-3600`; Playwright `medium-nav-tabs-keymap` cobre ⇧⇧ |
| 03 descoberta automática | 4 testes descobertos sem clicar em Discover ("Discovered 4 tests · exit 0") | Falha de descoberta com causa no painel; troca de projeto de teste: não rodados no app (falha de descoberta exigiria quebrar o fixture) |
| 04 stack trace clicável | Link `OrderTests.cs:29` abre o arquivo; "Ln 29, Col 1"; stack crua visível | Caso sem posição: não reproduzido na fixture (todos os resultados têm posição). Coberto pelo Playwright `tests` |
| 05 menus e Finder | Menu de arquivo (Open, Copy Path, Find in Files…, Show in Finder, Rename…, Delete); menu de projeto com Show in Finder; menu de solução (Build, Restore, Reload, Copy Path, Find in Files…, Show in Finder, Add Project… desabilitado); menu de diretório; Show in Finder selecionou `bin` no Finder | Motivo do "Add Project…" desabilitado: fica no atributo `title` (`solutionMenu.ts:131`), que não aparece na acessibilidade nem no screenshot. Coberto pelo Playwright `solution-explorer` (`toHaveAttribute("title", …)`) |
| 06 build clicável | Erro `CS1003` provocado na cópia isolada; Build falhou; link `Program.cs(3,36)` abre o arquivo em "Ln 3, Col 36" | — |
| 07 foco após navegação | ✓ Enter num arquivo novo pelo Search Everywhere (depois da correção) deixa o foco no editor e ⌘F abre o Find; ✓ ⌘F após clique numa referência do Find Usages e num resultado do Search Everywhere | Find in Files: não rodado no app |
| 08 contador do Find | Contador `n of m` (UI em inglês, como o resto do app); Enter avança (2→3→4 of 6); consulta vazia mostra `0 of 0` | Texto em inglês, não "n de m" como no ticket (decisão de UI, não corrigida). Teste de Playwright novo para Enter |
| 09 chords e run config | Settings com nota de chords; "Duplicate to edit" cria "Run OrderDesk.Cli copy" editável e some o aviso de somente leitura (via Run ▸ Edit run configurations…) | — |
| 10 próxima falha e Run Unit Test | Run all gera falha; "Next failure" abre o arquivo; "Run Unit Test" aparece no projeto de teste (não no `Cli`) e roda os 4 testes do projeto | Estado desabilitado sem falhas: não verificado |
| 11 breakpoint sem código | ✓ ⌘F8 com caret na linha 12 (em branco) cria o marcador vazado na linha (teclado, sem coordenada) | Texto "No code on this line…" (no `title` do marcador, não exposto na acessibilidade). Coberto pelo Playwright `breakpoint-no-code` |
| 12 chrome e rótulos | "Keymap: Rider (macOS)" na barra de status; botões com nome (Build, Run startup project, Debug, Search Everywhere, Open Settings); Monitoring, AI Assistant e Database com nome e desabilitados | Tooltip dos botões ausentes: hover físico sobre Monitoring não mostrou tooltip na captura (`12-tooltip-monitoring.png`) |

## Defeitos e achados desta rodada

- **Enter no Find não avança:** **não é defeito.** Pela sonda no Chromium e no app, o 1º Enter
  seleciona o match a partir do caret e o 2º avança. O teste que eu tinha escrito estava errado.
  Corrigido no spec (`editor-find-count`).
- **Foco após Enter no Search Everywhere (ticket 07):** defeito real no app, não corrigido (ver
  tabela).
- **Drawer de Workspace tools desatualizado após a instalação:** continua mostrando "No launcher at
  the destination" e "Reuse verified installation" depois de a análise ficar `Ready`. Não
  investigado nesta rodada.
- **Rótulo do menu de pasta:** a janela de menu aparece com título "Solution context menu" também
  para diretórios (só rótulo).

## Evidências (`docs/evidence/rider-parity/`)

Capturas e textos de árvore de acessibilidade das rodadas anteriores (`00`–`10`), e nesta rodada:
`05-solucao-add-project-hover.png`, `06-build-erro-link-program-cs-3-36.png`,
`09-duplicate-to-edit.png`, `12-tooltip-monitoring.png`, `08-find-contador-e-analise-ready.png`.

## Verify-openridr

- Superfície C (app real): executada conforme acima, com `desktop.sh up/down` e cleanup (sem
  processo restante, `lsof` vazio nas portas da E2E).
- Superfície B (navegador): **não executada**, a ferramenta de navegador não está disponível nesta
  sessão.

## Limitações aceitas (driver ou fixture), registradas

- Duplo Shift, tooltips nativos (inclui o motivo do Add Project e o texto do marcador de breakpoint),
  caso de stack trace sem posição e estado desabilitado do Run Unit Test. Cobertos por Playwright
  quando há teste; sem evidência visual no app.

## Pendências

1. Find in Files (ticket 07) no app real: não rodado.
2. Estado desabilitado do Run Unit Test sem falhas (ticket 10): não rodado no app.
3. Falhas conhecidas: `find-usages` (existe no `main`) e `run-configs` (2, do HEAD).
4. Instabilidade sob carga das 5 specs listadas (passam isoladamente): investigar o uso de
   estado compartilhado entre specs na mesma rodada.
5. Adaptador de debug (`DebugAdapterPanel`) tem o mesmo texto "No adapter at the destination."
   depois da instalação; não corrigido, fora do drawer de Workspace tools.

## Revisão (Sonnet) das correções 76b96e2 / cd6c325 / badc156

- **Foco após navegação por painel (item 1):** a regra de `focusEditorWhenMounted` voltou a valer
  também quando o foco está no controle que originou a navegação (linha de Find Usages, Problems,
  Tests). Só para em diálogo modal (`[role="dialog"][aria-modal="true"]`), no terminal e em campos de
  texto. Não para no editor, porque a primeira tentativa pode cair no editor antigo.
  - Testes Chromium: `focus-after-navigation` (5) e `find-usages` "Enter on a Find Usages reference…"
    passam. **Limitação:** no Chromium o editor já foca sozinho depois da navegação (medido: o foco
    vai para `div testid=editor`), então esse teste de teclado **não discrimina** a regressão. A prova
    que discrimina é o app real, pendente.
  - Achado de regressão pego pela rodada: a versão que retentava até o editor ter foco quebrava o ⌘E
    (`recent-files` perdia o foco). Corrigido com a regra acima.
- **Guarda de ⌘E após Enter (item 2):** teste novo "Recent Files opened right after a Search Everywhere
  Enter keeps the focus in its dialog" passa. Só espera a aba do arquivo, não o foco do editor, para
  disputar o foco com o laço. A versão anterior (que roubava o foco) o faria falhar.
- **Drawer (item 3), raiz no host:** `Plan::record_verified` em `crates/host/src/tools.rs` atualiza
  `plan.existing` depois de instalação verificada e de reuso. Teste Rust
  `tools_install_is_real_verified_and_reused`: **vermelho antes** (plano continuava `absent`:
  "No launcher exists at the destination."), **verde depois**. Override do frontend removido
  (`ToolchainPanels.tsx` voltou à versão anterior). E2E `csharp-ls` "install plan is exact…": passa
  sem o override.
- **Rust (crate `openridr-host`):** 221 + 14 + 80 + 9 + 5 + 1 passam, 0 falham.
- **Playwright, área de navegação/painéis (11 specs):** 60 passaram, 1 falhou
  (`find-usages` "a blocked Find Usages runs once": "csharp-ls did not answer the references request
  within 5 seconds" numa execução; passou na outra). Instabilidade de LSP, não de foco.

## Pendente de app real (aguardando liberação da sessão do desktop)

- Item 1 no app: Enter em linha do Find Usages (ou Problems/Tests) deve deixar o foco no editor e ⌘F
  abrir o Find; ⌘E logo após Enter deve manter o foco no diálogo. Não rodado (app em uso pela outra
  sessão).
