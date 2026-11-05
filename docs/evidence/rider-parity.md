# Evidência — Paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Plataforma: macOS ARM64.
Tickets 13–16 estão adiados e não foram verificados.

## Resumo

- **App Tauri real (superfície C, agent-desktop):** bundle de debug do checkout
  (`0.3.0-alpha.6`, igual ao `package.json`), em cópia isolada da fixture `OrderDesk`, com
  `csharp-ls` 0.28.0 instalado pelo próprio app (confirmado pelo app). Rodada com
  `--headed` autorizada pelo Helder: teclado, digitação, clique direito físico e hover.
- **Provado no app real:** 01 (parcial), 02 (parcial), 03, 04, 05 (menu da pasta e Show in
  Finder), 07 (parcial), 08 (parcial), 10 (parcial), 12 (parcial).
- **Sem evidência visual:** 06, 09 (Duplicate to edit), 11, tooltip do 12, duplo Shift e
  "Run Unit Test".
- **Playwright (superfície A):** 115 passaram, 2 falharam, 1 pulado, nos specs ligados aos
  tickets 01–12. As 2 falhas são de `run-configs.spec.ts`, na lista de falhas conhecidas do HEAD.
- **`find-usages` "lists the real references" (lsp-state "Partial coverage"):** **já existia no
  `main`** (`5c83aae`). Mesma falha, mesma asserção, no mesmo `csharp-ls`. Não é regressão desta
  branch.
- **Superfície B (navegador):** não executada. A ferramenta de navegador não está disponível
  nesta sessão.
- Nenhum defeito novo foi encontrado nesta rodada, então não houve correção de código.

## Ambiente do app real

- Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app` (`bun run tauri build --debug
  --bundles app`). `/Applications` não foi tocado.
- Subida e descida: `verify-openridr/scripts/desktop.sh up <bundle>` e `down`, com `OPENRIDR_HOME`
  isolado e cópia da fixture em `<tmp>`. Sem processo restante.
- Permissões: Acessibilidade `granted`; Gravação de Tela `granted`.
- Confiança: concedida só na cópia isolada da fixture.
- `csharp-ls` 0.28.0: instalado pelo botão "Confirm and install" do próprio app, com NuGet.org
  e SHA-256 registrado. Só dentro do home isolado.
- Entradas físicas: `press` (Escape, uma vez, para fechar um menu aberto por engano), e depois
  as teclas e cliques da rodada com `--headed`, autorizados pelo Helder.

## Achados

- O menu de pasta tem `aria` de título "Solution context menu", mesmo para um diretório. Só um
  rótulo, não afeta o comportamento.
- Após a instalação, a aba Workspace tools continua mostrando "No launcher at the destination"
  e o botão "Reuse verified installation". A barra de status mostra "Analysis: Ready". Parece
  um estado de drawer que não foi atualizado após a instalação. **Não investigado.**
- Contador do Find: a UI mostra `5 of 12` (inglês, como o resto do app). O ticket 08 escreve
  "n de m" em pt-BR, e o spec `editor-find-count` espera `1 of 3`. Mantive a UI como está.
- Enter no campo Find não avançou o contador (ainda `5 of 12`). **Não investigado.**
- Duplo Shift não abriu o Search Everywhere com `press shift` nem com `key-down`/`key-up`.
  O driver pode não entregar gesto de modificador isolado; o Playwright cobre ⇧⇧.

## Evidências (`docs/evidence/rider-parity/`)

| Arquivo | O que prova |
|---|---|
| `00-estado-inicial.png` | Janela após abrir a fixture: dica "⌘T or Double Shift", "Keymap: Rider (macOS)", "Install analysis" |
| `01-plano-instalacao-sem-confirmar.txt` | Plano do `csharp-ls` 0.28.0, "Plan ready — nothing downloaded yet" |
| `02-keymap-settings-arvore.txt` | Settings > Keymap: perfil Rider, chords fixos, ⌘T, ⇧F12, ⌘1…⌘8 |
| `02-find-usages-referencias-shift-f12.txt` | ⇧F12 real: "4 references from csharp-ls", `Program.cs (1)` e `OrderTests.cs (3)` |
| `03-tests-descoberta-automatica.txt` | "Discovered 4 tests · exit 0" sem clicar em Discover |
| `04-resultado-falha-run-all.txt` | Run all: "2 passed · 1 failed · 1 skipped", stack trace com `OrderTests.cs:line 29` |
| `04-editor-apos-link-ln29.txt` | Clique no link `OrderTests.cs:29`: "Ln 29, Col 1" |
| `04-link-stack-trace-ln29.png` | Screenshot do mesmo estado |
| `05-menu-pasta-show-all-files.txt` | Menu da pasta `bin`: Copy Path, Find in Files…, Show in Finder, New File…, Rename…, Delete |
| `08-find-contador-e-analise-ready.png` | Painel Find com `5 of 12` e "Analysis: Ready" |
| `10-next-failure-editor.txt` | "Next failure" abre `OrderTests.cs` na falha |

Show in Finder (ticket 05): o Finder abriu com a pasta `bin` selecionada (`Pasta [selected]`)
dentro da cópia isolada. A janela foi fechada depois.

## Por ticket

Legenda: **✓** provado no app real. **PW** = Playwright. **Sem evidência visual** = não
provado no app; o motivo está na coluna.

| Ticket | Gaps | App real | Playwright | Pendente e motivo |
|---|---|---|---|---|
| 01 análise | G01 | ✓ "Install analysis" abre o plano, sem download antes de confirmar; ✓ instalação pelo app; ✓ ⇧F12 com análise pronta lista as referências | PW `analysis-gate`, `install-plan`, e "a blocked Find Usages runs once after Start analysis" (passou 2 vezes) | "Start analysis" no app: a análise estava pronta e não parei para repetir o caminho. Rename e code actions no app: não rodados. Restricted no app: não rodado |
| 02 keymap e janelas | G02–G05 | ✓ Settings: perfil Rider, ⌘T, ⇧F12, ⌘1…⌘8, nota de chords; ✓ ⌘T abre Search Everywhere; ✓ ⌘1 alterna Solution Explorer (1→0→1); ✓ ⌘8 alterna Tests (1→0) | PW `medium-nav-tabs-keymap` | Duplo Shift: sem evidência visual (não reproduzido pelo driver). ⌘8 restaurado depois |
| 03 descoberta automática | G08 | ✓ 4 testes sem clicar em Discover | PW `tests-auto-discovery` | Falha de descoberta com causa no painel, e troca de projeto de teste: não rodados no app |
| 04 stack trace clicável | G09 | ✓ link abre o arquivo; "Ln 29, Col 1"; stack crua visível | PW `tests` | Caso sem posição: não rodado no app |
| 05 menus e Finder | G13–G15 | ✓ menu do diretório com os itens do ticket; ✓ Show in Finder seleciona `bin` | PW `solution-explorer` | Menu de arquivo, projeto e solução no app; Add Project desabilitado com motivo |
| 06 build clicável | G20 | Sem evidência visual | PW `build` | Exige erro de build no fixture (edição de código) |
| 07 foco após navegação | G06 | ✓ ⌘F após clicar numa referência do Find Usages abre o Find do editor na primeira vez | PW `focus-after-navigation` | Abrir pela busca de arquivos e Find in Files: não rodados no app |
| 08 contador do Find | G07 | ✓ painel mostra `5 of 12` para "Order" | PW `editor-find-count` | Enter não avançou (não investigado); `0 de 0` sem consulta: não rodado |
| 09 chords e run config | G11, G18 | ✓ Settings mostra os chords e a nota de que não são remapeáveis | PW `run-configs`, `settings` | "Duplicate to edit" no app: controle não localizado sem coordenadas |
| 10 próxima falha e Run Unit Test | G12, G17 | ✓ Run all gera falha; ✓ "Next failure" abre o arquivo da falha; "Rerun failed" presente | PW `tests`, `run` | "Run Unit Test" por tipo de projeto: não rodado no app |
| 11 breakpoint sem código | G19 | Sem evidência visual | PW `breakpoint-no-code` | Clique na margem precisa de coordenadas da linha em branco |
| 12 chrome e rótulos | G21–G23 | ✓ "Keymap: Rider (macOS)"; botões com nome (Build, Run startup project, Debug, Search Everywhere, Open Settings); Monitoring, AI Assistant e Database com nome e desabilitados | PW `shell`, `disabled-states`, `medium-nav-tabs-keymap` | Tooltip dos botões ausentes: sem evidência visual (hover não mostrou tooltip na captura) |

## Find Usages com `csharp-ls` (ticket 01)

- Rodada isolada de `tests/e2e/find-usages.spec.ts` (home de E2E com `csharp-ls`): "a blocked Find
  Usages runs once after Start analysis" passou nas duas execuções; "Find Usages lists the real
  references…" falhou nas duas com `lsp-state` em "Partial coverage" (espera "Ready").
- Comparação com `main` (`5c83aae`, alpha.5) em worktree temporário, mesmo `csharp-ls`, home de
  E2E separado: **mesma falha**. Worktree removido depois.
- No app real, com `csharp-ls` instalado e análise "Ready", ⇧F12 listou as 4 referências.

## Pendências

1. **Aceite restante no app:** 06, 09 (Duplicate to edit), 11, tooltips do 12, "Run Unit Test" do
   10, "Start analysis" do 01 (parar e iniciar a análise), e os menus de arquivo, projeto e
   solução do 05.
2. **Investigar:** Enter no Find não avança; drawer de Workspace tools desatualizado após a
   instalação; duplo Shift pelo driver.
3. **Falhas conhecidas:** `run-configs.spec.ts` (2). `find-usages` "Partial coverage" existe no
   `main`; investigação separada.
4. **Superfície B (navegador):** não executada.
