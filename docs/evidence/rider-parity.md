# Evidência — Paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Plataforma: macOS ARM64.
Bundle de debug do checkout (`0.3.0-alpha.6`), cópia isolada da fixture `OrderDesk`,
`csharp-ls` 0.28.0 instalado pelo próprio app no home isolado. Entrada física (`--headed`)
autorizada pelo Helder. Tickets 13–16 adiados, não verificados.

## Playwright (superfície A), specs dos tickets 01–12

- Rodada final: **116 passaram, 3 falharam, 1 pulado**.
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
| 03 descoberta automática | 4 testes descobertos sem clicar em Discover ("Discovered 4 tests · exit 0") | Falha de descoberta com causa no painel; troca de projeto de teste: não rodados no app |
| 04 stack trace clicável | Link `OrderTests.cs:29` abre o arquivo; "Ln 29, Col 1"; stack crua visível | Caso sem posição: não reproduzido na fixture (todos os resultados têm posição). Coberto pelo Playwright `tests` |
| 05 menus e Finder | Menu de arquivo (Open, Copy Path, Find in Files…, Show in Finder, Rename…, Delete); menu de projeto com Show in Finder; menu de solução (Build, Restore, Reload, Copy Path, Find in Files…, Show in Finder, Add Project… desabilitado); menu de diretório; Show in Finder selecionou `bin` no Finder | Motivo do "Add Project…" desabilitado: fica no atributo `title` (`solutionMenu.ts:131`), que não aparece na acessibilidade nem no screenshot. Coberto pelo Playwright `solution-explorer` (`toHaveAttribute("title", …)`) |
| 06 build clicável | Erro `CS1003` provocado na cópia isolada; Build falhou; link `Program.cs(3,36)` abre o arquivo em "Ln 3, Col 36" | — |
| 07 foco após navegação | ⌘F depois de clicar numa referência do Find Usages abre o Find na 1ª vez; ⌘F depois de clicar num resultado do Search Everywhere também funciona (foco em `textfield: Editor`) | **Defeito no app real:** abrir arquivo pelo Search Everywhere com **Enter** deixa o foco em `webarea`, e o ⌘F não abre o Find. Reproduzido duas vezes com teclas reais, editor já montado ou não. No Chromium o mesmo caminho passa. Causa não confirmada (hipótese: WKWebView e `focus()` depois de operação assíncrona). Não corrigido: sem reprodução no Playwright, não consegui escrever teste vermelho. Find in Files: não rodado |
| 08 contador do Find | Contador `n of m` (UI em inglês, como o resto do app); Enter avança (2→3→4 of 6); consulta vazia mostra `0 of 0` | Texto em inglês, não "n de m" como no ticket (decisão de UI, não corrigida). Teste de Playwright novo para Enter |
| 09 chords e run config | Settings com nota de chords; "Duplicate to edit" cria "Run OrderDesk.Cli copy" editável e some o aviso de somente leitura (via Run ▸ Edit run configurations…) | — |
| 10 próxima falha e Run Unit Test | Run all gera falha; "Next failure" abre o arquivo; "Run Unit Test" aparece no projeto de teste (não no `Cli`) e roda os 4 testes do projeto | Estado desabilitado sem falhas: não verificado |
| 11 breakpoint sem código | — | Clique na margem da linha 12 (em branco) com `mouse-click --headed` em duas coordenadas não colocou breakpoint (`Ln 1` inalterado). Coordenadas não calibradas no gutter. Coberto pelo Playwright `breakpoint-no-code` |
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

## Pendências

1. Foco após Enter no Search Everywhere (ticket 07): reproduzir no Playwright ou instrumentar o
   WebView antes de corrigir.
2. Gutter (ticket 11): calibrar a coluna de breakpoint no app real.
3. Tooltips (ticket 12), duplo Shift (ticket 02), motivo do Add Project (ticket 05): dependem de
   captura de tooltip/gesto que o driver não entrega.
4. Restricted mode (ticket 01) no app real.
5. Drawer de Workspace tools desatualizado após a instalação.
6. Falhas conhecidas: `find-usages` (existe no `main`) e `run-configs` (2).
