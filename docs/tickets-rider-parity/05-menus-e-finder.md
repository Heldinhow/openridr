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

- [x] Pasta abre menu com Novo arquivo, Copiar caminho, Find in Files, Renomear, Excluir e
      Show in Finder. — Playwright `solution-explorer.spec.ts` ("folder menu keeps the existing
      actions and adds Show in Finder") clica com o botão direito numa pasta e afirma os seis
      itens; o menu já existia e o handler da linha da pasta não precisou de correção.
- [x] Show in Finder funciona em arquivo, pasta, projeto e solução e seleciona o item. —
      host `workspace.reveal` (arquivo/pasta/raiz validados em teste Rust) e item nos quatro
      menus; o Playwright prova o caminho do item até o método e o erro de contenção. A seleção
      no Finder real fica para a varredura final.
- [x] Caminho fora do workspace é recusado pelo host; plataforma não suportada devolve erro. —
      Rust `reveal_resolves_inside_the_workspace_only` (`outside_workspace`, `not_found`) e
      `reveal_is_a_clear_error_off_macos` (`unsupported_platform`); Playwright posta um caminho
      externo no host real e recebe `outside_workspace` sem abrir o Finder.
- [x] Menu da solução tem Build Solution, Restore Solution, Copy Path, Find in Files e Add
      Project desabilitado com motivo. — Playwright "solution menu: …" afirma os itens; em
      Restricted, Build/Restore ficam desabilitados com o motivo do trust.
- [x] Nenhum item sem backend é exibido como habilitado. — Add Project é o único item sem
      backend e sai desabilitado com "Adding projects is not implemented."; Rebuild/Clean não
      existem.
- [ ] Evidência no app real, incluindo o Finder. **Pendente:** a passada com `agent-desktop`
      (clique direito em pasta, arquivo, projeto e solução; Show in Finder e item selecionado
      no Finder) fica para a varredura final com `verify-openridr`.

## Feito (2026-11-04)

- **G13 (reprodução):** o menu de pasta já abria com Novo arquivo, Copiar caminho, Find in
  Files, Renomear e Excluir; nenhum handler foi reescrito. A prova virou teste Playwright.
- **Host:** `workspace.reveal` em `crates/host/src/lib.rs` valida a contenção
  (`resolve_reveal_in_workspace`, aceita a raiz do workspace) e chama `fsutil::reveal_command`,
  que devolve `open -R <path>` no macOS e `unsupported_platform` no resto. Sem gate de trust
  (não entra em `EXECUTABLE_METHODS`). A UI mostra `Show in Finder failed: <mensagem>`.
- **UI:** `src/solutionMenu.ts` ganhou `onReveal` em `fileMenuItems`/`projectMenuItems` e o novo
  `solutionMenuItems` (Build Solution com o gate de `build.solution`, Restore Solution,
  Reload Solution, Copy Path, Find in Files, Show in Finder e Add Project desabilitado), ligado
  em `treeMenuItems` para a raiz. `src/host/client.ts` não mudou: `call` já é genérico.
- **Testes:** Rust `cargo test -p openridr-host --lib` 219/219 (3 novos); Playwright
  `solution-explorer.spec.ts` + `medium-nav-find-in-files.spec.ts` 16/16; `bun run check`.
- **Pendente no app real (agent-desktop):** o roteiro de aceite acima; a varredura final com
  `verify-openridr` fecha este item.

