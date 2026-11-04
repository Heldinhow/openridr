# Evidência — Paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Plataforma: macOS ARM64.
Tickets 13–16 estão adiados e não foram verificados.

## Resumo

- **App Tauri real (superfície C, agent-desktop):** rodou com o bundle de debug do checkout
  (`0.3.0-alpha.6`, igual ao `package.json`), em cópia isolada da fixture `OrderDesk`. Provado
  por árvore de acessibilidade, texto de tela e screenshot em 01, 02, 03, 04, 10 e 12. Os
  demais itens dependem de teclado (`--headed`) ou de coordenadas e ficaram sem evidência visual.
- **Playwright (superfície A):** 115 passaram, 2 falharam, 1 pulado, nos specs ligados aos
  tickets 01–12. As 2 falhas são de `run-configs.spec.ts`, na lista de falhas conhecidas do HEAD.
- **Falha de `find-usages` "lists the real references" (lsp-state "Partial coverage"):**
  **já existia no `main`** (`5c83aae`, release alpha.5). Mesma falha, mesma asserção, rodada
  isolada com o mesmo `csharp-ls` 0.28.0.
- **Superfície B (navegador):** não executada. A ferramenta de navegador não está disponível
  nesta sessão.
- Nenhum ticket foi marcado como verificado de ponta a ponta. Os itens provados no app estão
  marcados abaixo.

## Ambiente do app real

- Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, gerado com
  `bun run tauri build --debug --bundles app`. `/Applications` não foi tocado.
- Subida: `verify-openridr/scripts/desktop.sh up <bundle>`, com `OPENRIDR_HOME` isolado e
  cópia de `fixtures/OrderDesk` em `<tmp>`. Encerrada com `desktop.sh down` (sem processo
  restante).
- Permissões: Acessibilidade `granted`; Gravação de Tela `granted`.
- Confiança: concedida só na cópia isolada da fixture, pelo diálogo do app.
- Ação de teclado: uma tecla Escape enviada com `press` (CGEventPost, sem `--headed`) para
  fechar o menu do WebKit aberto por engano na árvore de pasta. Foi a única entrada física.

## Capturas e textos (`docs/evidence/rider-parity/`)

| Arquivo | O que prova |
|---|---|
| `00-estado-inicial.png` | Janela após abrir a fixture: dica do editor com "⌘T or Double Shift", barra de status com "Keymap: Rider (macOS)" e "Install analysis" |
| `01-plano-instalacao-sem-confirmar.txt` | Plano de `csharp-ls` 0.28.0 com "Plan ready — nothing downloaded yet" e o botão "Confirm and install" sem clique |
| `02-keymap-settings-arvore.txt` | Settings > Keymap com perfil `Rider (macOS)`, nota de chords fixos, Search Everywhere ⌘T, Find Usages ⇧F12, janelas ⌘1…⌘8 |
| `03-tests-descoberta-automatica.txt` | Aba Tests com "Discovered 4 tests · exit 0" sem clique em Discover |
| `04-resultado-falha-run-all.txt` | Run all: "4 results · 2 passed · 1 failed · 1 skipped", stack trace com `OrderTests.cs:line 29` |
| `04-editor-apos-link-ln29.txt` | Após clicar no link de stack trace: "Ln 29, Col 1" |
| `04-link-stack-trace-ln29.png` | Screenshot do mesmo estado |

## Por ticket

Legenda: **App ✓** = provado no app real nesta rodada. **PW** = Playwright. **Sem evidência
visual** = não provado no app; o motivo está na coluna.

| Ticket | Gaps | App real | Playwright | Pendente e motivo |
|---|---|---|---|---|
| 01 análise | G01 | ✓ "Install analysis" na barra de status; painel abre; plano mostrado sem download | PW `analysis-gate`, `install-plan`, e "a blocked Find Usages runs once after Start analysis" (passou 2 vezes) | "Start analysis" e consulta pendente no app: não rodado (exigiria instalar `csharp-ls` no home do app). Rename e code actions no app: não rodados. Restricted no app: não rodado |
| 02 keymap e janelas | G02–G05 | ✓ Settings: perfil Rider, ⌘T, ⇧F12, ⌘1…⌘8, nota de chords; dica do editor com Double Shift | PW `medium-nav-tabs-keymap` | Gestos de teclado (Double Shift, ⌘T, ⌘1, ⌘8) no app: sem evidência visual, precisam de `--headed` |
| 03 descoberta automática | G08 | ✓ árvore preenchida com 4 testes sem clicar em Discover | PW `tests-auto-discovery` | Falha de descoberta com causa no painel, e troca de projeto de teste: não rodados no app |
| 04 stack trace clicável | G09 | ✓ link `tests/OrderDesk.Tests/OrderTests.cs:29` abre o arquivo; "Ln 29, Col 1"; stack crua visível | PW `tests` | Caso sem posição: não rodado no app |
| 05 menus e Finder | G13–G15 | Sem evidência visual: clique direito em pasta abriu o menu padrão do WebKit, e o menu customizado não apareceu na árvore | PW `solution-explorer` | Menu de pasta, arquivo, projeto e solução no app; Show in Finder (seleção no Finder) |
| 06 build clicável | G20 | Sem evidência visual | PW `build` | Exige erro de build no fixture: edição de texto precisa de `--headed` |
| 07 foco após navegação | G06 | Sem evidência visual | PW `focus-after-navigation` | ⌘F após abrir pela busca precisa de teclado |
| 08 contador do Find | G07 | Sem evidência visual | PW `editor-find-count` | Digitar no Find precisa de `--headed` |
| 09 chords e run config | G11, G18 | ✓ Settings mostra os chords com a nota de que não são remapeáveis | PW `run-configs`, `settings` | "Duplicate to edit" no app: não localizei o controle da configuração derivada sem coordenadas |
| 10 próxima falha e Run Unit Test | G12, G17 | ✓ Run all gera falha; "Next failure" abre o arquivo da falha (`OrderTests.cs`); "Rerun failed" presente | PW `tests`, `run` | "Run Unit Test" por tipo de projeto e estado desabilitado sem falhas: não rodados no app |
| 11 breakpoint sem código | G19 | Sem evidência visual | PW `breakpoint-no-code` | Clique na margem do editor precisa de coordenadas (`--headed`) |
| 12 chrome e rótulos | G21–G23 | ✓ barra de status "Keymap: Rider (macOS)"; botões com nome (Build, Run startup project, Debug, Search Everywhere, Open Settings); Monitoring, AI Assistant e Database com nome e desabilitados | PW `shell`, `disabled-states`, `medium-nav-tabs-keymap` | Tooltip dos três botões ausentes: não aparece na árvore, só no hover (`--headed`) |

## Find Usages com `csharp-ls` (ticket 01)

- `csharp-ls` 0.28.0 presente no home de E2E (`tools/csharp-ls`).
- Rodada isolada de `tests/e2e/find-usages.spec.ts` com o reporter `list`, 2 execuções:
  - "a blocked Find Usages runs once after Start analysis": **passou** nas duas.
  - "Find Usages lists the real references of Order and opens one": **falhou nas duas** com
    `lsp-state` em "Partial coverage" (espera "Ready", timeout de 90 s).
- **Comparação com `main`:** worktree temporário em `main` (`5c83aae`, alpha.5), mesmo
  `csharp-ls`, home de E2E separado, mesmo teste. Falhou com a mesma asserção. **Conclusão:
  a falha é anterior a esta branch.** Worktree removido depois do teste.
- O teste pulado da rodada completa de Playwright continua não identificado: nesta execução
  separada, os dois testes de `find-usages` rodaram sem pular.

## Pendências

1. **Teclado e coordenadas no app real:** atalhos (Double Shift, ⌘T, ⌘1, ⌘8, ⌘F), digitação
   (Find, edição para erro de build), clique na margem (breakpoint) e hover (tooltips). Exigem
   `--headed`, que move o cursor e o foco do Mac. Não rodei sem sua permissão explícita.
2. **Itens sem ref na árvore:** o disclosure "Details" dos resultados de teste e o combobox
   da configuração de execução não têm ref semântico.
3. **Start analysis no app real:** precisa instalar `csharp-ls` no home do app isolado. A
   instalação faria download de NuGet, então não foi executada.
4. **verify-openridr superfície B:** não executada, sem ferramenta de navegador.
5. **Falhas conhecidas:** `run-configs.spec.ts` (2) e `find-usages` "Partial coverage" (1,
   confirmada também no `main`). Não investigadas além disso.
