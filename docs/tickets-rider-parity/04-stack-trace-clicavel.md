# 04 — `arquivo:linha` clicável na stack trace (G09)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Testes". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Clicar em `arquivo:linha` na saída de um teste abre o arquivo na linha, como no Rider.

## Pontos de código

- `src/TestsPanel.tsx`: bloco de detalhe do resultado, `result.stackTrace`, `result.position`
  e `onNavigate`.
- `src/WorkspaceView.tsx`: `navigateTo` (já normaliza o caminho e posiciona o editor).
- `src/host/client.ts`: tipo `TestResult.position`.

## Comportamento

- Quando `position` existe, o detalhe mostra um botão em forma de link com o texto exato
  `arquivo:linha` (linha 1-based). Clicar abre o arquivo na linha e coluna reportadas.
- A stack trace crua continua visível; o link não substitui o texto.
- Sem `position` dentro do workspace, permanece a mensagem honesta atual ("The source reported
  no `in <file>:line <n>` position inside the workspace...") e nenhum link falso.
- O botão "Open source position" atual é substituído pelo link, sem duplicar controle.

## Teste e verificação

- Playwright: no caso de teste falho já existente, afirmar que existe um botão com o texto
  `...:line N`, clicar e conferir a aba/linha ativa no editor.
- App real (agent-desktop): rodar OrderDesk.Tests com uma falha, abrir Details, clicar no link
  e conferir o cursor na linha.

## Aceite

- [x] `arquivo:linha` na stack trace é clicável e abre o arquivo na linha. — Playwright
      `tests/e2e/tests.spec.ts`: link `tests/OrderDesk.Tests/OrderTests.cs:29` abre a aba e
      deixa o cursor em `28:0` (linha 1-based exibida, 0-based interna).
- [x] A stack trace crua continua visível. — o `<pre>` de `test-result-stack-*` não mudou e o
      mesmo caso continua afirmando `OrderTests.cs:line 29`.
- [x] Sem posição no workspace, nenhum link é inventado e a mensagem atual permanece. — o ramo
      sem `position` continua renderizando `test-result-no-position-*` sem controle de navegação.
- [ ] Evidência no app real. **Pendente:** a passada com `agent-desktop` (rodar OrderDesk.Tests
      com a falha, abrir Details, clicar no link e conferir o cursor) fica para a varredura
      final com `verify-openridr`.

## Feito (2026-11-04)

- `src/TestsPanel.tsx`: o botão "Open source position" virou um `<button className="link-button">`
  com o texto exato `position.path:line` (linha exibida 1-based, `position.line + 1`), mantendo
  `data-testid="test-result-navigate-<index>"` e chamando o `onNavigate`/`navigateTo` existente
  com a linha e a coluna reportadas. A stack trace crua e o ramo sem posição não mudaram.
- `tests/e2e/tests.spec.ts`: o caso de execução real afirma o texto exato do link e clica nele;
  a asserção da stack crua continua antes do clique.
- `bun run check`; Playwright `tests/e2e/tests.spec.ts` 7/7 com dotnet 8.0.408 real.
- **Pendente no app real (agent-desktop):** o roteiro de aceite acima; a varredura final com
  `verify-openridr` fecha este item.
