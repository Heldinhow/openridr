# 07 — Foco no editor após navegação (G06)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Navegação e Find do editor". **Bloqueado
por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Depois de abrir um arquivo por busca, find in files, recentes, find usages, definição ou link
de teste, ⌘F abre a busca do editor na primeira tentativa. Clique simples na árvore continua
com o foco na árvore.

## Pontos de código

- `src/WorkspaceView.tsx`: `openSearchFile`, `openSearchLocation`, `focusEditor`,
  `RecentFilesDialog`, `FindUsagesPanel`, `TestsPanel` e o handler de clique da árvore.
- `src/CodeEditor.tsx`: efeito de `target` (já foca quando há linha/coluna).

## Comportamento

- A causa atual é o `focusEditor()` antes do `openUserFile`; o foco cai antes de o editor
  montar. Focar depois que o arquivo abre (uma vez, no próximo tick) resolve os caminhos de
  navegação.
- Caminhos cobertos: Search Everywhere (arquivo e localização), Find in Files, recentes,
  Find Usages, go-to-definition e link de teste.
- Clique simples na árvore não rouba o foco; o duplo clique/abertura por teclado pode seguir o
  comportamento atual.
- Não criar um segundo handler global de teclado; usar o `focusEditor` existente.

## Teste e verificação

- Playwright: abrir `Order.cs` pela busca, pressionar ⌘F e afirmar que o painel de busca do
  editor está visível; repetir por Find in Files e por um link de teste; clicar na árvore e
  afirmar que o foco não foi para o editor.
- App real (agent-desktop): abrir pela busca, ⌘F; clicar na árvore, ⌘F (não deve abrir) e
  então clicar no editor, ⌘F (deve abrir).

## Aceite

- [ ] Abrir arquivo pela busca e pressionar ⌘F abre a busca do editor na primeira vez.
- [ ] Find in Files, recentes, Find Usages, definição e link de teste também devolvem o foco.
- [ ] Clique simples na árvore mantém o foco na árvore.
- [ ] Evidência no app real.
