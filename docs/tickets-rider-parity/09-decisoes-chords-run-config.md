# 09 — Decisões: chords e configuração derivada (G11, G18)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Configuração derivada e chords". **Bloqueado
por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Fechar G11 e G18 com decisão registrada e o menor ajuste de UX que remove o atrito, sem
inventar um modelo novo de persistência ou de keymap.

## Pontos de código

- `src/runConfigs.tsx`: aviso `run-config-default-note`, `derived`, `duplicate()`.
- `src/actions/KeymapDialog.tsx` e `src/SettingsDialog.tsx`: exibição de `chordLabels`.
- `docs/decisions.md`: decisões da grill; completar com o motivo de não remapear chord e de
  não editar a derivada diretamente.

## Comportamento

- **G18:** a configuração derivada continua somente leitura. O aviso ganha um botão
  "Duplicate to edit" que chama o `duplicate()` existente, deixa a cópia editável e mantém a
  derivada intacta. Não criar store de overrides por solução/contexto.
- **G11:** os chords ⌘U+R/D/L/F continuam. A Settings mostra os chords e uma nota de que não
  são remapeáveis; o override por atalho único continua funcionando. Não implementar editor de
  chord.
- As duas decisões ficam em `docs/decisions.md` com o motivo e o que faltaria para mudar.

## Teste e verificação

- Playwright: abrir a configuração derivada, clicar em "Duplicate to edit", conferir que a
  cópia é editável e que a derivada segue somente leitura; abrir a Settings e conferir os
  chords e a nota.
- App real (agent-desktop): repetir os dois fluxos e conferir o foco/estado dos botões.

## Aceite

- [ ] O aviso da configuração derivada tem "Duplicate to edit" e a cópia fica editável.
- [ ] A configuração derivada continua somente leitura.
- [ ] A Settings mostra os chords e a nota de que não são remapeáveis.
- [ ] Um override por atalho único continua funcionando.
- [ ] As duas decisões estão em `docs/decisions.md`.
- [ ] Evidência no app real.
