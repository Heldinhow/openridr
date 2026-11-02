# Retro — Solution Explorer fiel ao Rider (tickets 38–41)

## O que funcionou
- Ler os 4 prints antes de tocar no código e fechar o escopo por "tem backend?" evitou menu falso.
- Reaproveitar `EditorContextMenu`, os gates puros de `executionGates.ts` e `grantTrust` manteve Restricted/Trust idêntico ao resto do app.
- Normalizar modelo avaliado e outline na mesma `View` deu uma árvore só.

## O que deu errado
- Colisão `SolutionTree.tsx` × `solutionTree.ts` (macOS não diferencia caixa): helper renomeado para `solutionNodes.ts`.
- O teste de Dependencies dependia do `<details>` antigo (por role/texto, não por testid); reescrito para o nó novo.
- Disco cheio no meio da suíte fez restore falhar; depois da limpeza, `run-configs.spec.ts:139/171` seguem falhando **também no HEAD sem estas mudanças** (build falha no ambiente) — pré-existente, não relacionado.
- Cache do `cargo` com binário do worktree de baseline: o host sem `workspace.tree` serviu o E2E. Lição: após rodar baseline no mesmo `target/`, dar `touch` nos fontes do host.
- O menu dentro do `<nav>` rolável deslocava o painel ao focar; `createPortal` (modo compact) resolveu.

## Code-review (2 eixos) — tratado
- Corrigido: guard de trust também com config explícita (`requestBuild`/`prepareAndDebug`); `onClose` inline reexecutava o foco do menu (ref); foco entra no submenu com →; locate abre subpastas aninhadas; `.csproj` não aparece na raiz do Show All; ⋮ ganhou Expand/Collapse; spec alinhada (`hidden`, config explícita, Dependencies plano).
- Não tratado (baixo valor/risco): erro de `workspace.tree` vira lista vazia sem aviso; corrida ao desligar Show All durante carga; fonte linkada fora da pasta do projeto em `nestSources`; duplicação `sourceRows`/`diskRows`; `TreeRow.children` não usado.

## Fica de fora
Add/Unload/EF/Run Multiple/Publish/Refactor/Inspect/Reformat/Diagrams/Tools, Cut/Copy/Paste/Delete/Rename, Open In, Replace in Files, Scratches and Consoles, Dependencies agrupado, `.gitignore` real em `no index`, verificação no app Tauri.
