# 12 — Chrome: rótulo do keymap, tooltips e ícones (G21, G22, G23)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Chrome". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

A barra de status mostra o perfil ativo, as ferramentas ausentes explicam por que não abrem e
todo ícone da Solution tem nome acessível e tooltip.

## Pontos de código

- `src/shell/StatusBar.tsx`: `status-keymap` (hoje "Keymap: IntelliJ / Rider").
- `src/WorkspaceView.tsx`: faixa direita (`tool-windows-right`) com Monitoring, AI Assistant e
  Database.
- `src/SolutionToolbar.tsx`: `aria-label` e `title` dos ícones.

## Comportamento

- G21: `status-keymap` mostra `Keymap: Rider (macOS)` ou `Keymap: IntelliJ`, conforme o perfil
  ativo (`profileTitle`). Nada de rótulo ambíguo.
- G22: os três botões ausentes continuam desabilitados, com `aria-label` e tooltip que dizem
  que não fazem parte do OpenRidr. Se a solução for ocultá-los, o tooltip deixa de existir e o
  teste passa a afirmar a ausência; a escolha registrada é manter com tooltip.
- G23: cada botão da faixa da Solution tem `aria-label` e `title`; conferir os sete (mostrar
  todos, buscar, localizar, expandir, recolher, opções, ocultar).

## Teste e verificação

- Playwright: afirmar o texto de `status-keymap` nos dois perfis; afirmar `aria-label`/`title`
  dos três botões ausentes e dos sete da Solution.
- App real (agent-desktop): trocar o perfil na Settings e conferir a status bar; hover nos
  botões ausentes e nos ícones da Solution.

## Aceite

- [ ] A status bar mostra o nome exato do perfil ativo.
- [ ] Os três botões ausentes têm nome acessível e tooltip explicando que não fazem parte.
- [ ] Os sete ícones da Solution têm `aria-label` e `title`.
- [ ] Evidência no app real.
