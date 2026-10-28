# Reteste de navegação — merge `ui-redesign` em `main`

Data: 28/10/2026. Branch `main`: merge `d2267fe` (`merge: integrate ui-redesign into main`) +
`f2253cd` (adaptação do spec de aceitação ao shell compacto).

Este documento registra o reteste manual-assistido da navegação, como um usuário, depois do
merge do redesign. Ele não amplia a matriz de aceitação: nenhum status mudou (ver o fim).

## Ambiente e método

| Item | Valor observado |
|---|---|
| SO / hardware | macOS 27.2, Apple M2, arm64 |
| Host | `cargo run -p openridr-host-http -- --port 4317` (host HTTP real) |
| UI | `bun run dev` → Vite em `http://localhost:1420` (modo web, sem janela Tauri) |
| Navegador | Chrome 155 via Playwright 1.64.1, `channel: "chrome"`, headless |
| Viewports | 1440×900 e 1100×700 (execuções separadas, cada uma com a jornada completa) |
| Fixture | cópia temporária de `fixtures/OrderDesk` (`.sln`, 3 projetos, 4 testes), sem `bin/obj` |
| App home | o real (`~/Library/Application Support/OpenRidr`), como um usuário; workspace fechado no fim |

Cada execução começou com `workspace.close` no host para ver a tela de boas-vindas. Prints em
`docs/evidence/nav-retest/<viewport>-<passo>.png`; o texto abaixo resume o observado.

## Passos executados

| # | Passo (como usuário) | 1440×900 | 1100×700 | Observado |
|---|---|---|---|---|
| 01 | Abrir o app sem workspace | **ok** | **ok** | Tela "Open a solution" com campo de caminho, aviso de Restricted mode e cartões da Toolchain (`Analysis host ✓ SDK 10.0.100`, `Debug adapter ! Setup required`). |
| 02 | Abrir solução pela tela inicial | **ok** | **ok** | `OrderDesk.sln` aberto; cartão de trust com caminho completo; Toolchain expandida mostra host 10.0.100 e projeto `8.0.408` (após trust). |
| 03 | Confiar no workspace | **ok** | **ok** | Checkbox + "Trust and inspect project"; projeto resolve para SDK 8.0.408 com `global.json rollForward: disable`. |
| 04 | Entrar no workspace e avaliar | **ok** | **ok** | Árvore avaliada: Cli (console, net8.0), Core (library), Tests (test); `Evaluated · revision 1`; nenhum `bin/obj` listado. |
| 05 | Explorer: expandir e abrir arquivos | **ok** | **ok** | Projetos já vêm expandidos pela avaliação; `Program.cs` e `Order.cs` abrem em abas distintas. |
| 06 | Editor e abas | **ok** | **ok** | Cursor no status (`Ln 13, Col 1` / `Ln 9, Col 6`), troca de aba por clique, marcador de dirty ausente. |
| 07 | Painéis inferiores | **ok** | **ok** | Build, Problems, Debug, Terminal e Tests alternam; conteúdo e foco da aba corretos; Problems mostra "coverage partial" com o motivo. |
| 08 | Redimensionar explorer e painel de saída | **ok** | **ok** | Separadores acessíveis por teclado: sidebar 280→328 px, saída 220→188 px; layout estável. |
| 09 | Restore explícito | **ok** | **ok** | `Restore succeeded · exit 0`, comando, cwd e SDK visíveis; "3 projects report restored assets". |
| 10 | Reload da avaliação + build | **ok** | **ok** | `revision 2` e `Build succeeded · exit 0`; atalho exibido por SO (`Cmd+F9`). |
| 11 | Run da build salva | **ok** | **ok** | `Run exited`; log contém `Order A-104: R$ 42,00`. |
| 12 | Descoberta e execução de testes | **ok** | **ok** | `Discovered 4 tests`; execução com 2 passed, 1 failed, 1 skipped (estado esperado da fixture). |
| 13 | Debug | **bloqueado** | **bloqueado** | `Debug Not started` + motivo textual: adaptador Apple Silicon experimental e desabilitado por padrão (AC-23). Sem instalação/launch. |
| 14 | Settings | **n/a** | **n/a** | Não existe superfície de settings nesta entrega (nem botão, nem link); nada a navegar. |
| 15 | Fechar e reabrir workspace | **ok** | **ok** | "Close workspace" no menu `•••` volta à tela inicial; reabrir o mesmo `.sln` volta em Restricted mode, sem herdar trust. |
| — | Erros de página (console) | **nenhum** | **nenhum** | `pageerror` vazio nas duas execuções. |

## Correções pequenas feitas por causa do merge

`f2253cd fix(acceptance): drive the compact shell in the P0 journey and UI audit`

O spec `tests/e2e/zz-acceptance.spec.ts` (ticket 18, escrito em `main`) ainda assumia o shell
antigo, sempre visível. Ajustes, sem afrouxar asserções:

1. abrir o disclosure dono antes de clicar em ações do workspace, restore/reload, LSP e revoke
   (mesmo helper `reveal` usado pelas outras suítes);
2. abrir e fechar o inspetor de projeto, que é um overlay e cobria o painel de restore;
3. aceitar o rótulo de build por SO (`Cmd+F9`/`Ctrl+F9`) que o redesign adotou;
4. na auditoria de UI, ignorar conteúdo de `<details>` fechado (não está na árvore renderizada)
   e aceitar a origem `127.0.0.1:1420` do bridge same-origin `/__host`.

Nenhuma dessas mudanças altera comportamento do app; são adaptações do teste ao desenho novo.

## Pendências e observações (não corrigidas)

- **Debug bloqueado**: limitação já conhecida e documentada (AC-23/AC-24), não é regressão do
  merge. O painel explica o motivo e não oferece instalação enquanto a flag experimental estiver
  desligada.
- **Inspetor de projeto é overlay**: com ele aberto, cobre a lateral direita do painel inferior
  (inclusive o botão de restore). Fechar o inspetor resolve; é o comportamento desenhado, mas o
  usuário precisa descobrir o botão "Close project details". Candidato a ajuste de UX futuro.
- **Rótulos longos no explorer a 1100 px**: a linha do projeto corta a última tag
  (`OrderDesk.Tests`); não impede expandir nem abrir arquivos. Cosmético.
- **Settings**: não existe na entrega; a SPEC menciona "settings ampliadas" apenas como P1.

## Gates executados no merge

| Gate | Resultado |
|---|---|
| `bun install --frozen-lockfile` | sem mudanças (96 installs / 157 packages) |
| `bunx tsc --noEmit` | exit 0 |
| Lint | não há linter configurado no repositório (nenhum arquivo de config; nada a rodar) |
| `cargo test --workspace` | 195 testes, 0 falhas (118+9+54+9+5) |
| `bun run test:e2e` | 53 testes, 53 passed (4,4 min) |

## Matriz de aceitação

Nenhum status de `docs/acceptance-matrix.md` mudou: AC-32 já estava `approved com limitações`
(sem leitor de tela e sem janela Tauri nativa), e o reteste reforça a mesma evidência em
1440×900 e 1100×700 sem provar o que falta. AC-23/AC-24 seguem como estavam (debug experimental
bloqueado). O documento não foi editado para não criar evidência que ele não registra.
