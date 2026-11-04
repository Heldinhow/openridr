# 02 — Keymap, Search Everywhere e janelas de ferramenta (G02, G03, G04, G05)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Keymap". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Fechar as divergências de atalho com decisão registrada e devolver os atalhos que faltam:
Search Everywhere e as oito janelas de ferramenta.

## Pontos de código

- `src/actions/keymap.ts`: `keymapDefaults`, `chordTable`, perfis `rider-macos` e `intellij`.
- `src/actions/KeymapDialog.tsx` e `src/SettingsDialog.tsx`: exibição e captura de atalho.
- `src/WorkspaceView.tsx`: registro das ações `toolWindow.*`, handler global de teclado,
  `toggleToolWindow` e o estado de layout.
- `src/actions/FindAction.tsx` e a dica da tela inicial do editor: texto do atalho.
- `docs/decisions.md`: entrada da grill já registrada; completar com a pendência do keymap base
  se necessário.

## Comportamento

- `rider-macos`: Find Usages continua ⇧F12; `intellij`: continua ⌥F7. Nada muda no código,
  só a prova e a decisão.
- Símbolo continua ⌥⌘⇧T no `rider-macos`; o mapeamento de Ctrl+Shift+Alt+N do keymap base para
  macOS fica registrado como não confirmado.
- Search Everywhere abre com duplo Shift e com ⌘T nos dois perfis. O atalho aparece na
  Settings e na dica da tela inicial.
- `toolWindow.solution` … `toolWindow.tests` ganham ⌘1…⌘8 na ordem da faixa (Solution, Commit,
  Build, Run, Problems, Debug, Terminal, Tests). O mesmo atalho alterna: abre, foca e fecha.
  Git e NuGet ficam sem padrão, mas remapeáveis.
- Nenhum atalho novo pode conflitar com os existentes; a Settings mostra conflito como hoje.

## Teste e verificação

- Playwright: `tests/e2e/medium-nav-tabs-keymap.spec.ts` (ou um spec novo de keymap) confere
  os rótulos na Settings, a abertura de Search Everywhere por ⌘T e o toggle de uma janela por
  ⌘1; o duplo Shift é testado por eventos de teclado no editor.
- App real (agent-desktop): abrir Search Everywhere com duplo Shift e com ⌘T; abrir e fechar
  Solution com ⌘1; abrir Tests com ⌘8; conferir os atalhos na Settings.

## Aceite

- [ ] ⇧F12 (rider-macos) e ⌥F7 (intellij) continuam funcionando e aparecem na Settings.
- [ ] Duplo Shift e ⌘T abrem Search Everywhere; o atalho aparece na Settings e na dica.
- [ ] ⌘1…⌘8 alternam as oito janelas de ferramenta, sem conflito, e aparecem na Settings.
- [ ] O símbolo continua no atalho atual do perfil, com a pendência do keymap base registrada.
- [ ] Evidência no app real para duplo Shift, ⌘T, ⌘1 e ⌘8.
