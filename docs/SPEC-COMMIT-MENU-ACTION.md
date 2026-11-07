# SPEC — O item Commit… do menu de workspace actions abre o painel de Commit (issue #190)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#190](https://github.com/Heldinhow/openridr/issues/190) (achado ao converter o menu na #177).

## Problem Statement

O item **Commit…** do menu de workspace actions (`data-testid="git-commit-action"`) não tem `onClick`. Com Git disponível e workspace confiado ele fica **habilitado** e clicar nele não faz nada: nem abre o painel de Commit, nem fecha o popover. A ação que ele deveria executar já existe na mesma registry que alimenta o keymap (⌥⇧⌘K), a Find Action e o menu nativo — `git.commit` → `openCommitPanel()` —, então o defeito é só a fiação do botão, perdida quando o menu deixou de ser `<details>` (o `closest("details")?.removeAttribute("open")` dos vizinhos virou `close()` e este item ficou sem handler nenhum).

O caso habilitado nunca foi exercitado: o único teste que olha para o botão (`tests/e2e/actions.spec.ts`) abre o workspace **restrito** e só assere o estado desabilitado. A evidência da #177 registrou `'Commit…' (disabled)` pelo mesmo motivo — a fixture copiada não é repositório Git.

## Solution

O item passa a chamar `close()` e executar a ação `git.commit`, como os outros itens do mesmo menu: o popover fecha, o foco volta ao gatilho e o painel de Commit abre na barra lateral esquerda. O estado desabilitado (Git bloqueado ou workspace restrito) não muda — mesmo `title`, mesmo `disabled`.

## User Stories

1. Como pessoa com Git disponível num workspace confiado, quero que clicar em **Commit…** abra o painel de Commit, para não precisar procurar o botão da barra lateral nem decorar o atalho.
2. Como pessoa com Git disponível num workspace confiado, quero que o popover feche ao clicar em **Commit…**, para ele não ficar por cima do editor.
3. Como pessoa que usa só teclado, quero que o foco volte ao gatilho do menu depois de escolher **Commit…**, para não cair no `body`.
4. Como pessoa num workspace restrito ou com o Git bloqueado, quero que **Commit…** continue desabilitado com o mesmo motivo no `title`, para saber por que não posso commitar.
5. Como pessoa que usa a Find Action ou o atalho ⌥⇧⌘K, quero o mesmo painel de Commit de hoje, para o menu não virar um segundo caminho com comportamento diferente.
6. Como pessoa que mantém o OpenRidr, quero `data-testid="git-commit-action"` mantido, para nenhum teste ou receita de verificação quebrar por renomeação.
7. Como pessoa que valida a release, quero prova no app Tauri real dos dois critérios (habilitado abre e fecha; desabilitado mantém o motivo), para saber que vale no WKWebView e não só no código.

## Implementation Decisions

- **Conserto por item, com o `close()` que o `ToolbarMenu` já entrega.** O `onClick` chama `close()` e depois `commitAction.execute()`, exatamente como `build-solution-menu`, `open-find-action`, `open-keymap` e `sdk-open-global-json`. Auto-fechar em qualquer `<button>` do popover foi recusado na #177 e continua recusado: os `<select>` de configuração/TFM e o `sdk-redetect` não fecham hoje e não passam a fechar.
- **A ação não muda.** `git.commit` continua definida uma vez na registry (`execute: () => openCommitPanel()`), servindo keymap, Find Action, menu nativo e agora o botão. Nada de lógica nova no componente.
- **Estado desabilitado intocado.** `disabled={commitAction.disabledReason !== null}` e `title={commitAction.disabledReason ?? commitAction.title}` ficam como estão; `gitBlockedReason` continua devolvendo a mensagem de trust para workspace restrito.
- **`close-workspace` e `sdk-redetect` ficam como estão.** O primeiro fecha o workspace inteiro (o menu desmonta com ele) e o segundo foi deixado aberto de propósito na #177 para mostrar o inventário recém-detectado. Fora do escopo desta issue.
- **`data-testid` são contrato e ficam todos**: `git-commit-action` no item, `workspace-actions-menu` no container do menu, `commit-tool` no painel que abre.
- **Sem dependência nova, sem `data-testid` novo, sem mudança de CSS.**

## Testing Decisions

- **Nenhum teste automatizado novo, e nenhuma execução de Playwright.** Decisão do Helder no grill desta issue: o critério "um teste cobre o caso habilitado" da issue é **substituído** pela prova no app Tauri real. Motivos: (a) a mudança é uma linha de fiação sem seam puro — não há função a extrair que valha um teste (ao contrário do `branchTriggerLabel` da #168); (b) o repo não tem infraestrutura de DOM e não se adiciona dependência para isso; (c) Playwright é smoke noturno desde a #67, nunca verificação de sessão.
- **A prova é o app real dirigido por `agent-desktop`**, cobrindo os dois critérios de aceite visíveis: com Git e trust, clicar em **Commit…** abre o painel de Commit e o popover sai da árvore; com workspace restrito, o item continua `disabled` com o mesmo `title`. Registrada em `docs/evidence/commit-menu-action.md`.
- **O que precisa continuar verde**: `bun run check` (`tsc --noEmit` + `cargo check`) e `bun test tests/unit` — a mudança não toca nenhum módulo puro.
- **O caso desabilitado que já existia em `tests/e2e/actions.spec.ts` não é tocado** (o spec continua válido; só não é executado nesta sessão).
- **Não verificado por esta superfície**: leitor de tela real (VoiceOver), ordem de tabulação, Windows e macOS Intel.

## Out of Scope

- Fechar o popover em `sdk-redetect` ou em `close-workspace`.
- Revisar os outros itens do menu ou a registry de ações.
- Qualquer mudança no painel de Commit, no host ou nas mensagens do Git.
- Cenário E2E novo ou execução de Playwright.

## Further Notes

- Origem: a #177 converteu o menu de workspace actions em `ToolbarMenu` (botão + render prop `close()`) e o item `Commit…` ficou sem handler; a evidência daquela frente já registrava `'Commit…' (disabled)` porque a fixture copiada não é repositório Git.
- A conversão da #177 chegou em `origin/main` (`9abc5e4`) enquanto esta sessão começava; o trabalho foi feito em cima dela, não do `<details>` anterior.
