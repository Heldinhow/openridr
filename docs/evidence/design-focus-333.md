# Foco visível no rail de tool windows (#333)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1440×900 e 1100×760.

## O que mudou

`src/styles.css`: `.toolband-button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }`.

O rail tem 34 px de largura, o botão tem 32 px e o rail rola (`overflow-y: auto`, que também recorta
no eixo horizontal). Com o anel global (`outline-offset: 2px`), o anel de 2 px ocupa 4 px para fora do
botão e é cortado nas duas laterais. Inset, ele fica inteiro dentro do botão.

Geometria medida (1440 e 1100, mesmos valores):

| Item | Valor |
|---|---|
| botão | `x = 5`, largura `32` (5…37) |
| rail | `x = 4`, largura `34` (4…38) |
| anel antes | offset `+2` → 1…41, cortado em 4 e 38 |
| anel depois | offset `-2`, largura `2` → 5…37, inteiro dentro do rail |

## Verificação

- Tab percorre **11 itens** do rail nas duas larguras (`toolband-solution`, `commit`, `build`, `run`,
  `problems`, `debug`, `terminal`, `tests`, `git`, `nuget` e `workspace-tools-toggle`), e todos
  reportam `outline-offset: -2px`, `outline-width: 2px`, `outline-style: solid`,
  `outline-color: rgb(84, 138, 247)`, com o botão inteiro dentro do rail nos quatro lados.
- Contraste do anel `--accent` sobre o fundo do rail (`--chrome` `#17181a`) = **5,38:1**; sobre o
  botão ativo (`--accent-surface-strong` sobre o chrome, `#2a2b2d`) = **4,34:1** — acima dos 3:1
  exigidos para indicador de foco.
- Auditoria de nome acessível: **nenhum** botão do rail sem `aria-label` (inclusive os desabilitados
  "M/A/D"), nas duas larguras.
- `data-testid` do rail preservados (`toolband-*`, `tool-windows`, `tool-windows-right`).
- `tests/e2e/rail-focus.spec.ts` — **1 passed**: Tab percorre os 11 itens nas duas larguras e, com
  cada um focado, mede `outline-offset: -2px`, `outline-width: 2px`, `solid`, dentro do rail nos
  quatro lados, com `aria-label`; ao fim, audita todo botão do rail (inclusive desabilitado) por
  `aria-label`.

## Não verificado / decisão

- **Rótulo curto persistente:** avaliadas as três saídas e descartadas duas. (a) Alargar o rail de
  34 para ~72 px: muda a coluna do grid (`34px` fixo) e rouba ~38 px do editor em toda tela — o
  oposto do que o #331 pediu. (b) Rótulo sob o ícone dentro dos 34 px: não cabe ("Solution
  Explorer" precisa de ~90 px) e dobraria a altura do rail, que já rola a 760 px. (c) Mantida a
  largura, com `aria-label` + `title` em todo item e o cabeçalho inferior (mesmo destino) com
  rótulo persistente. É uma decisão de layout, não uma omissão.
- App Tauri real (macOS) e árvore de acessibilidade do WebKit: fora desta máquina.
- Os três botões desabilitados "M/A/D" do rail direito continuam como estão: são placeholders de
  recurso inexistente, já com `aria-label` e `title` explicando o motivo.

## Capturas

`docs/evidence/design-focus-333/before-rail-focus.webp` e `after-rail-focus.webp` (recorte do rail
com o foco de teclado em Solution Explorer).
