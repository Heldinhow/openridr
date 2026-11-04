# Retro: polimento visual (ícones e fontes)

Spec: [SPEC-UI-POLISH.md](SPEC-UI-POLISH.md). Tickets: [tickets-ui-polish/](tickets-ui-polish/README.md). Notas: `implementation-notes.md` (seção "Polimento visual"). Decisões: `decisions.md` (2026-11-04).

## O que mudou

- Lucide (`lucide-react`) no lugar dos 38 paths à mão; glifo C# do MDI neutro; selos CSS removidos; ícones de arquivo neutros.
- Inter Variable e JetBrains Mono Variable (latin) embutidas; ligaduras off; escala em 4 tokens; alvos 32×32 na toolbar da árvore.
- `THIRD-PARTY-NOTICES.md` cobre Lucide (ISC + MIT/Feather), MDI (Apache-2.0), Inter e JetBrains Mono (OFL-1.1) com os textos.

## Bundle (`bun run build`, medido)

| | antes | depois |
|---|---|---|
| JS | 1.287,92 kB (gzip 374,58) | 1.308,59 kB (gzip 378,96) |
| CSS | 103,25 kB (gzip 19,48) | 106,01 kB (gzip 19,65) |
| woff2 | 0 | 131,6 kB (Inter 48,3 + Mono 40,4 + Mono itálico 43,0) |

Os 38 ícones custaram ~18,5 kB de JS na 1ª medição (≈0,5 kB cada) e o resto vem de `@font-face`/escala. Total de assets ≈ +154 kB. O `.app` de debug (52,98 MiB) não serve de comparação com release.

## Evidência visual (app Tauri real, build debug local, `screencapture -x`)

Antes: `evidence/ui-polish/ws-*.png` (Gabagool). Depois (fixture OrderDesk, home isolada): `depois-1-aberto.png`, `depois-2-editor.png`, `depois-3-crop-toolbar.png`, `depois-4-crop-tree.png`, `depois-5-settings.png`, `depois-6-settings-editor.png`.

Visto de verdade: toolbar (martelo, Play e Debug verdes, busca, engrenagem) legível; árvore com C# neutro, pasta/solução/dependências em Lucide e linha selecionada; aba com glifo C#; Settings (primeira captura da tela) com "JetBrains Mono" como default; editor em JetBrains Mono; UI em Inter.

## Critérios do spec

1. sem texto-ícone: `grep cs-badge|json-badge src` vazio. ✔
2. um conjunto + glifo C#: ✔ (`grep 'd="' src/Icon.tsx` vazio).
3. traço 1,5 px: aplicado por CSS; **não medido em pixel**.
4. reconhecíveis a 100%: ✔ nas capturas; alvo ≥ 32×32 inspecionado só no CSS, **não no DOM**.
5. árvore distinguível: ✔ para projeto, `.cs`, pasta, solução, dependências; `.csproj`/`.sln`/`.json` **não verificados** (OrderDesk não os mostra).
6. `document.fonts`: **não verificado** (WKWebView sem console); só a prova visual de Inter/JetBrains Mono renderizando no app embutido.
7. sem font-size < 11 px nem rem fracionário: ✔ (grep vazio).
8. itálico real e sem ligadura: configurado; **não inspecionado** em captura.
9. contraste WCAG e foco por anel: **não medido**.
10. `check`, `build` ✔. E2E: 244 passaram, 9 falharam. Conhecidas do AGENTS.md: find-usages, 2× run-configs, 2× tests. Demais: `ui-fold 1100x700` e `csharp-ls:1646` passaram ao rodar isoladas (flaky); `ui-layout:115` e `ui-layout:153@1440` **falham também no HEAD sem as mudanças** (worktree de comparação) e variam entre execuções; `zz-acceptance` P0 falhou em `tree-file-…/Program.cs` (nó OrderDesk.Cli recolhido na árvore, 15 min de timeout; mesmo sintoma do `ui-layout:115`, que falha no HEAD) — **não comparado no HEAD** por custo (15 min); a auditoria de UI do mesmo arquivo (`:858`, contraste, sem requisição externa) passou.
11. NOTICES: ✔; o arquivo está em `Contents/Resources` do `.app` construído. ✔
12. Não capturado: sessão de debug ativa (barra de passos), `.csproj/.sln/.json`.

## Lições

- Ícones do Lucide têm o tamanho vindo do CSS no app inteiro; `absoluteStrokeWidth` seguiria o prop `size` e erraria. `non-scaling-stroke` resolve num lugar só.
- O pacote Fontsource emite todos os subsets no `dist`; um `@font-face` latin próprio evita ~centenas de kB.
- `sed -i` do macOS exige sufixo; usar Python para edições em lote.
- O E2E completo leva ~37 min e tem falhas não determinísticas fora da lista do AGENTS.md (`ui-layout`, `ui-fold`, `csharp-ls`): comparar com o HEAD antes de culpar a mudança; atualizar a lista em `RETRO-DEVFLOW`.
- Pendências: capturar a barra de passos com sessão de debug real e revisar `redo-2`/`arrow-*-to-line`; propor ao DESIGN.md (não alterado aqui) fixar Lucide + glifo C#, os 4 tokens `--fs-*` e Inter/JetBrains Mono como embutidas.
