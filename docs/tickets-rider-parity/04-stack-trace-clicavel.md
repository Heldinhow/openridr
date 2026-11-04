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

- [ ] `arquivo:linha` na stack trace é clicável e abre o arquivo na linha.
- [ ] A stack trace crua continua visível.
- [ ] Sem posição no workspace, nenhum link é inventado e a mensagem atual permanece.
- [ ] Evidência no app real.
