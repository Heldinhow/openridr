# 05 — Menus da árvore e da solução + Show in Finder (G13, G14, G15)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Solution e menus". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Fechar os menus de solução, projeto, pasta e arquivo e levar qualquer um desses itens ao
Finder, sem reescrever o que já funciona.

## Pontos de código

- `src/solutionMenu.ts`: `fileMenuItems`, `projectMenuItems`.
- `src/SolutionTree.tsx`: `onContextMenu` de arquivo, pasta, projeto e raiz.
- `src/WorkspaceView.tsx`: `treeMenuItems`, cópia de caminho, abertura de Find in Files,
  criação/renomeação/exclusão, ações de build/restore.
- `crates/host/src/lib.rs`: tabela de métodos do `dispatch`.
- `crates/host/src/fsutil.rs` (ou módulo novo pequeno): validação de contenção e argv de reveal.
- `src/host/client.ts`: tipo do novo método.

## Comportamento

- **G13 primeiro:** reproduzir o clique direito numa pasta no app real. O código atual já monta
  Novo arquivo, Copiar caminho, Find in Files, Renomear e Excluir. Se abrir, o ticket só prova e
  adiciona "Show in Finder"; se não abrir, corrigir o handler da linha da pasta. Não reescrever
  o menu por suposição.
- **G14:** novo método de host `workspace.reveal` com o caminho validado dentro do workspace.
  No macOS executa `open -R`; em plataforma não suportada devolve erro claro, sem fingir
  sucesso. Item "Show in Finder" em arquivo, pasta, projeto e solução. Sem gate de trust:
  abrir o Finder não executa código do projeto. "Open with" fica adiado; não enumerar apps.
- **G15:** menu da solução ganha Build Solution, Restore Solution, Copy Path, Find in Files e
  Add Project desabilitado com motivo ("adding projects is not implemented"). Rebuild/Clean
  ficam adiados; não criar item sem backend.
- Nenhuma dependência nova (nada de `tauri-plugin-opener`).

## Teste e verificação

- Rust: caminho dentro do workspace aceito; fora, recusado; plataforma não suportada devolve
  erro; argv esperado no macOS. Sem abrir o Finder nos testes automatizados.
- Playwright: menu de pasta com os itens; menu da solução com Build/Restore/Add Project;
  "Show in Finder" chama o método (stub/probe) e o erro de contenção aparece.
- App real (agent-desktop): clique direito em pasta, arquivo, projeto e solução; conferir o
  menu; usar Show in Finder e ver o item selecionado no Finder.

## Aceite

- [ ] Pasta abre menu com Novo arquivo, Copiar caminho, Find in Files, Renomear, Excluir e
      Show in Finder.
- [ ] Show in Finder funciona em arquivo, pasta, projeto e solução e seleciona o item.
- [ ] Caminho fora do workspace é recusado pelo host; plataforma não suportada devolve erro.
- [ ] Menu da solução tem Build Solution, Restore Solution, Copy Path, Find in Files e Add
      Project desabilitado com motivo.
- [ ] Nenhum item sem backend é exibido como habilitado.
- [ ] Evidência no app real, incluindo o Finder.
