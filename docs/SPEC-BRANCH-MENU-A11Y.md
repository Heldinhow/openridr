# SPEC — Gatilho do menu de branches como botão com nome acessível (issue #168)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#168](https://github.com/Heldinhow/openridr/issues/168) (follow-up do retro da #114).

## Problem Statement

O menu de branches é, desde a #114, o **único** caminho para **Complete merge** e **Abort merge…**. O gatilho desse menu é um `<summary>` de `<details>`, e o WKWebView o expõe na árvore de acessibilidade como `disclosure`, `interactive: false`, **sem `ref_id`** (medido no retro da #114 e na tabela de falhas do `test-app-agent-desktop`). Na prática: quem usa leitor de tela ou uma ferramenta dirigida por acessibilidade não vê o gatilho como ação, não consegue abrir o menu, e portanto não alcança as duas ações do merge. O nome que existe hoje (`Current branch: … · upstream … · N ahead, M behind`) também não diz que há um merge em andamento.

## Solution

O gatilho do menu de branches passa a ser um **`<button>` real** com `aria-expanded`, no mesmo padrão dos outros menus acessíveis do app (`RowMenu`, `SolutionToolbar`). O nome acessível vem de uma função pura e, durante um merge, diz que há um merge em andamento e que ele se conclui ou aborta ali. As duas ações continuam dentro do popover e passam a ser alcançáveis por papel e nome, porque agora existe um botão para abrir o popover.

## User Stories

1. Como pessoa que usa leitor de tela, quero que o gatilho do menu de branches seja um botão com nome, para saber que ele existe e que abre um menu.
2. Como pessoa que usa leitor de tela, quero ouvir **a branch atual** no nome do gatilho, para saber onde estou sem abrir nada.
3. Como pessoa que usa leitor de tela, quero ouvir o **upstream** e os contadores de ahead/behind, para saber se estou à frente ou atrás.
4. Como pessoa que usa leitor de tela, quero ouvir que **há um merge em andamento**, para saber que a árvore está no meio de um merge.
5. Como pessoa que usa leitor de tela, quero ouvir **onde** o merge se conclui ou aborta, para não procurar um terminal.
6. Como pessoa que usa só teclado, quero abrir o menu pelo gatilho e ouvir se ele está aberto, para navegar sem mouse.
7. Como pessoa que usa só teclado ou leitor de tela, quero alcançar **Complete merge** e **Abort merge…** por papel e nome, para agir no merge.
8. Como pessoa que usa só teclado, quero que **Escape** feche o menu e devolva o foco ao gatilho, para não me perder.
9. Como pessoa que usa o app com mouse, quero que **clicar fora** continue fechando o menu, para o comportamento não mudar.
10. Como pessoa que mantém o OpenRidr, quero o texto do nome acessível numa **função pura testada**, para o caso de merge não regredir sem ninguém ver.
11. Como pessoa que valida a release, quero **prova no app Tauri real** (gatilho como botão com `ref_id`, nome com merge, `aria-expanded`, e as duas ações por nome), para saber que vale no WKWebView e não só no código.
12. Como pessoa que mantém o OpenRidr, quero o mesmo defeito dos outros menus `<details>` da toolbar **registrado como follow-up**, para ninguém achar que todos foram consertados.

## Implementation Decisions

- **Gatilho é `<button type="button">` com `aria-expanded`.** Sem `aria-haspopup`: o popover não é um `menu` (tem busca, formulário de nova branch e lista), e declarar `menu` prometeria navegação por setas que não existe. `aria-expanded` é o que diz aberto/fechado.
- **O nome acessível é uma função pura exportada**, alimentada pelo status do Git, e é a **mesma string** de `aria-label` e `title`: uma fonte só para o leitor de tela e o tooltip. Ela mantém o prefixo `Current branch: …` (o teste E2E que já existe procura esse prefixo) e, quando a operação em andamento é merge, acrescenta ` · merge in progress — complete or abort it here`. Operações que o app não conclui (rebase, cherry-pick, revert) mantêm o texto de hoje; o popover já explica que esses casos pedem terminal.
- **O popover deixa de ser `<details>`.** O estado React que já existia (`menuOpen`) passa a controlar a montagem; saem o `onToggle`, o `removeAttribute("open")` e a ref do `<details>`. O caminho da command palette que abre o menu (`intent`) continua igual, e fica mais simples: nenhum DOM nativo para sincronizar.
- **Fechar é o de antes, sem o `<details>`.** Escape fecha e devolve o foco ao gatilho; clicar fora fecha. O handler global do workspace só casa `details.toolbar-menu`, então o componente ganha o seu próprio listener de clique fora. Fechar o menu continua limpando erro e o formulário de nova branch.
- **CSS sem regra nova de aparência.** Os seletores `.toolbar-menu > summary` passam a `:is(summary, button)`, e o botão herda o visual do gatilho atual. Nenhuma cor, medida ou token novo.
- **`data-testid` são contrato e ficam todos**: `branch-menu` (agora no container do popover), `toolbar-branch`, `branch-current`, `branch-upstream`, `branch-merge`, `branch-merge-note`, `branch-merge-complete`, `branch-merge-abort`, `branch-search`, `branch-error`, `branch-item-<nome>`, `branch-name`, `branch-create`. O papel do cabeçalho inferior e os `role="tab"` não são tocados.
- **Sem dependência nova e sem Playwright novo.** Decisão do Helder: a prova automatizada é `bun test` sobre a função pura; a prova de UI é o app real dirigido por `agent-desktop`.
- **Os outros três `<details>` da toolbar ficam fora** (projeto, workspace actions, run config). Eles têm o mesmo defeito de árvore de acessibilidade, mas não são caminho para nenhuma ação exclusiva; a issue filha de follow-up registra o caso.

## Testing Decisions

- **Bom teste aqui lê o texto que o leitor de tela ouve, não o DOM.** Por isso o seam é a função pura do nome acessível, no ponto mais alto possível sem infraestrutura de DOM: `bun test` em `tests/unit/`, no mesmo molde dos testes puros que já existem em `src` (ex.: `gitRefresh`, `solutionGitStatus`).
- **Casos cobertos**: sem merge com e sem upstream, com contadores de ahead/behind, detached HEAD, sem branch, e **merge em andamento** (o caso da issue). O caso do merge é o que falha antes da mudança e passa depois.
- **Sem teste de componente.** O repo não tem jsdom/happy-dom/testing-library e não se adiciona dependência para isso.
- **A prova de que o gatilho é acionável é o app real**, com `agent-desktop`: o nó do gatilho aparece como botão **com `ref_id`**, o nome contém `merge in progress`, `aria-expanded` alterna ao abrir/fechar, e **Complete merge** / **Abort merge…** são alcançáveis por papel e nome. É o que fecha o "a test covers it" da issue junto com o `bun test`.
- **Não verificado por esta superfície**: ordem de tabulação (o `agent-desktop` só vê `webarea` no conteúdo web), leitor de tela real (VoiceOver), Windows e macOS Intel.

## Out of Scope

- Os outros três menus `<details>` da toolbar (issue filha de follow-up).
- `role="menu"`/`menuitem`, navegação por setas e foco inicial dentro do popover.
- Qualquer mudança nas ações do merge, no host ou nas mensagens do Git.
- Reescrita da suíte Playwright; nenhum cenário E2E novo.

## Further Notes

- Origem: follow-up "Acessibilidade do menu de branches" do [RETRO-GIT-MERGE-IN-PROGRESS](RETRO-GIT-MERGE-IN-PROGRESS.md), que mediu o `<summary>` sem `ref_id` e precisou de clique físico em coordenadas para abrir o menu.
- A tabela de falhas conhecidas do `.agents/skills/test-app-agent-desktop` tem a linha "Disclosure … sem `ref_id`"; depois desta frente, o gatilho do menu de branches deixa de ser um caso dela.
