# SPEC — Os outros três menus da toolbar como botões com nome acessível (issue #177)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#177](https://github.com/Heldinhow/openridr/issues/177) (follow-up da #168, que consertou só o menu de branches).

## Problem Statement

Três menus da toolbar continuam sendo `<details>`/`<summary>`: **projeto** (soluções recentes), **workspace actions** (`⋮`) e **run configuration**. O WKWebView expõe o `<summary>` na árvore de acessibilidade como `disclosure`, `interactive: false`, **sem `ref_id`** — o mesmo defeito que a #168 mediu no menu de branches. Na prática: quem usa leitor de tela ou uma ferramenta dirigida por acessibilidade não vê esses três gatilhos como ação e não consegue abrir nenhum deles por papel e nome.

A #168 deixou esses três de fora porque nenhum deles é o único caminho para uma ação exclusiva. O defeito é o mesmo, e a evidência da #168 já o registrou no mesmo snapshot que provou o conserto do branch menu.

## Solution

O gatilho de cada um dos três menus passa a ser um **`<button type="button">` real** com `aria-expanded`, abrindo o popover que hoje é o conteúdo do `<details>`. Clicar fora e Escape continuam fechando; Escape devolve o foco ao gatilho. O visual não muda: o mesmo CSS que já trata `:is(summary, button)` para o menu de branches passa a valer para os três.

O estado de aberto/fechado, o listener de clique-fora e o handler de Escape ficam num **componente único** usado pelos três menus, em vez de três cópias do mesmo código.

## User Stories

1. Como pessoa que usa leitor de tela, quero que o gatilho do menu de projeto seja um botão com nome, para saber que ele existe e que abre uma lista de soluções recentes.
2. Como pessoa que usa leitor de tela, quero que o gatilho de workspace actions seja um botão chamado "Workspace actions", para alcançar configuração, TFM, Build Solution, Find Action, keymap, SDK, commit e fechar workspace.
3. Como pessoa que usa leitor de tela, quero que o gatilho de run configuration seja um botão com o nome da configuração ativa, para saber qual configuração Run e Debug vão usar.
4. Como pessoa que usa leitor de tela, quero ouvir se cada um desses menus está aberto ou fechado, para não abrir duas vezes nem ficar procurando conteúdo que não está lá.
5. Como pessoa que usa só teclado, quero abrir cada menu pelo gatilho, para navegar sem mouse.
6. Como pessoa que usa só teclado, quero que **Escape** feche o menu e devolva o foco ao gatilho, para não me perder.
7. Como pessoa que usa o app com mouse, quero que **clicar fora** continue fechando o menu, para o comportamento não mudar.
8. Como pessoa que usa o app com mouse, quero clicar no gatilho de novo para fechar o menu, para o toggle continuar como hoje.
9. Como pessoa que usa o app com mouse, quero que escolher uma solução recente, uma ação ou uma configuração continue fechando o menu, para o popover não ficar por cima do editor.
10. Como pessoa que usa o app, quero que a aparência dos três gatilhos seja exatamente a de hoje (tamanho, cor, posição, destaque quando aberto), para a correção de acessibilidade não virar mudança visual.
11. Como pessoa que mantém o OpenRidr, quero o comportamento de abrir/fechar num **único lugar**, para não repetir três vezes a mesma lógica e não reintroduzir o bug de hook condicional que deixou a janela em branco na #168.
12. Como pessoa que mantém o OpenRidr, quero todos os `data-testid` com os mesmos nomes, para nenhum teste ou receita de verificação quebrar por renomeação.
13. Como pessoa que mantém o OpenRidr, quero que os locators de E2E que apontam para `<summary>` sejam atualizados na mesma mudança, para o smoke noturno não quebrar por causa da troca de elemento.
14. Como pessoa que valida a release, quero **prova no app Tauri real** de que cada gatilho aparece como botão com `ref_id`, abre o popover e responde a Escape e clique fora, para saber que vale no WKWebView e não só no código.

## Implementation Decisions

- **Um componente compartilhado, molde do `RowMenu`.** Os três menus ganham o mesmo estado (`open`), a mesma ref do wrapper, a mesma ref do gatilho, o mesmo `useEffect` de clique-fora e o mesmo `close()` que devolve o foco. O componente é o único sítio de hooks: o `RunConfigWidget` tem `return` antecipado no caso desabilitado e, se o estado morasse nele, seria a **mesma armadilha de hook condicional** que deixou a janela em branco na #168 (registrada no retro daquela frente). Com o componente, a armadilha deixa de existir por construção.
- **O popover é um render prop que recebe `close()`.** Os seis botões internos que hoje fecham o menu via `closest("details")?.removeAttribute("open")` passam a chamar `close()`, e esse `close()` devolve o foco ao gatilho (o mesmo `pick()` do `RowMenu`): sem isso, o botão focado é desmontado e o foco cai no `body`. **Abrir `global.json` passa a fechar o menu** — era a única ação do popover que deixava o menu aberto por cima do editor, e a fatia que a move já declara isso. Auto-fechar em qualquer `<button>` do popover foi recusado: hoje os `<select>` de configuração/TFM e o botão de redetecção de SDK **não** fecham o menu, e isso não muda.
- **`pointerdown`, não `mousedown`.** É o evento que o menu de branches já escuta (#168) e o que o handler global do workspace usava; manter o mesmo evento mantém a semântica de "clique fora" idêntica entre os quatro menus.
- **O popover é montado condicionalmente** (`{open && …}`), como no #168: `aria-expanded` passa a refletir o DOM e nenhum spec asserta a presença do popover fechado. Sem fechar no Tab (o `BranchWidget` não fecha; o `RowMenu` fecha — manter o de hoje).
- **Sem `aria-haspopup`**, pelo mesmo motivo da #168: o popover não é um `menu` (tem `<select>`, formulário e lista), e declarar `menu` prometeria navegação por setas que não existe. `aria-expanded` é o que diz aberto/fechado.
- **Escape no wrapper, foco de volta ao gatilho.** O handler continua no container (como no `<details>` de hoje), então Escape só chega quando o foco está dentro do popover — comportamento inalterado. Clicar no gatilho aberto fecha.
- **Abrir o menu de run configuration continua disparando o refresh de launch profiles.** O `onToggle` de hoje vira chamada explícita no clique de abertura; nada de `useEffect` novo.
- **O caso desabilitado do run configuration não muda.** Quando não há configuração disponível, o widget já renderiza um `<button disabled>` dentro de `.toolbar-disabled` — não é disclosure e não tem defeito. O `return` antecipado fica onde está.
- **Nomes acessíveis idênticos aos de hoje**, sem função pura nova: `Project: <nome>. Open recent solutions.`, `Workspace actions` (com `title="More actions"`, como está), `Run configuration: <nome>. Run and Debug use it.`. A issue é sobre o papel do elemento; mudar texto ou tooltip é mudança visível.
- **`data-testid` são contrato e ficam todos**: `toolbar-project`, `toolbar-project-menu`, `workspace-actions-menu`, `run-config-menu`, `toolbar-run-config`, e todos os internos (`recent-solution-<n>`, `eval-configuration`, `eval-tfm`, `build-solution-menu`, `open-find-action`, `open-keymap`, `sdk-redetect`, `sdk-open-global-json`, `git-commit-action`, `close-workspace`, `run-config-active`, `run-config-option-*`, `run-config-profile-*`, `run-config-edit`). Nenhum renomeado, nenhum removido.
- **O handler global de clique-fora do workspace sai.** Ele só casa `details.toolbar-menu[open]`; depois desta mudança não casa nada (o menu de branches já virou `div` na #168). Cada menu passa a ter o seu listener, como o menu de branches.
- **CSS sem regra nova de aparência.** As regras por menu que ainda dizem `> summary` (projeto, run config, e as variantes de largura em media query) passam a `> :is(summary, button)`, no mesmo molde que a #168 aplicou às regras genéricas. Nenhuma cor, medida ou token novo; `font-weight: inherit` já está na regra base.
- **O `BranchWidget` não migra para o componente.** O `menuOpen` dele é aberto de fora pela command palette (`intent`) e o `closeMenu` limpa erro e formulário de nova branch; caber no componente exigiria modo controlado e hook de fechamento — API nova sem ganho, em código que acabou de ser provado.
- **Sem dependência nova e sem Playwright novo.** A prova de UI é o app Tauri real dirigido por `agent-desktop`, como na #168.

## Testing Decisions

- **Nenhum teste automatizado novo.** Não existe seam puro nesta mudança: os nomes acessíveis não mudam (não há função pura a testar, ao contrário do `branchTriggerLabel` da #168) e o que muda é ciclo de vida de componente. O repo não tem jsdom/happy-dom/testing-library e não se adiciona dependência para isso. A lacuna fica declarada como "não verificado".
- **Bom teste aqui leria o que o leitor de tela ouve e o que o clique faz, não o DOM.** Sem infraestrutura de DOM, esse teste não existe hoje; o que fecha o critério de aceite é a prova no app real.
- **O que precisa continuar verde**: `tsc --noEmit` (via `bun run build`/`check`) e `bun test tests/unit` — a mudança não toca nenhum módulo puro, então a suíte deve passar sem alteração.
- **Locators de E2E atualizados na mesma mudança, sem rodar Playwright.** Dez sítios usam `getByTestId("workspace-actions-menu").locator("summary")`; passam a `getByRole("button", { name: "Workspace actions" })`. É atualização mecânica de contrato de DOM (lição do retro da #168: ao trocar o elemento de um widget, `grep` no contrato de DOM), não cenário novo. Não executado nesta sessão: Playwright é smoke noturno (decisão da issue #67) — declarado como não verificado.
- **A prova de que o gatilho é acionável é o app real**, com `agent-desktop`: os três gatilhos aparecem como `button` **com `ref_id`** (antes: `disclosure` sem `ref_id`), `aria-expanded` alterna ao abrir/fechar, o popover abre por papel e nome, Escape devolve o foco ao gatilho e o clique físico fora fecha.
- **Não verificado por esta superfície**: ordem de tabulação (o `agent-desktop` só vê `webarea` no conteúdo web), leitor de tela real (VoiceOver), Windows e macOS Intel.

## Out of Scope

- Migrar o `BranchWidget` da #168 para o componente compartilhado (motivo acima).
- Os outros `<details>` que não são menus da toolbar (Build details, Run details, Test run details, logs do NuGet, Installation details, Setup log). Não são gatilho de menu e não têm o defeito medido.
- `role="menu"`/`menuitem`, navegação por setas e foco inicial dentro do popover.
- Qualquer mudança nas ações, no host ou nos textos dos menus.
- Reescrita da suíte Playwright; nenhum cenário E2E novo.

## Further Notes

- Origem: o snapshot da evidência da #168 mostrou `Project: …`, `Run configuration: …` e `Workspace actions` ainda como `disclosure` sem `ref_id`, o que virou a issue #177.
- A tabela de falhas conhecidas do `.agents/skills/test-app-agent-desktop` tem a linha "Disclosure ('Project: OrderDesk.sln') sem `ref_id`"; depois desta frente ela deixa de valer para os menus da toolbar.
