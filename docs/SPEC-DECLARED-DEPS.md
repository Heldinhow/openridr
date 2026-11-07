# SPEC — Dependências importadas passam a ser declaradas (issue #223)

Follow-up de #142. Medição original: `knip` em 2026-11-07, base `39c4c17`.

## Problem Statement

Três pacotes são importados direto pelo código da UI e **não** estão no `package.json` da raiz:
`@lezer/highlight` (o `tags` do highlighting), `@codemirror/search` (`getSearchQuery`, `gotoLine`,
`searchPanelOpen`) e `@codemirror/autocomplete` (`autocompletion`, `insertCompletionText`,
`pickedCompletion` e os tipos `Completion*`). Hoje eles resolvem porque chegam por transitividade de
`codemirror`, que é declarado. Isso é uma **dependência não listada**: o bump de um terceiro pode
removê-la e quebrar o build sem nenhum PR deste repositório tocar no assunto — o build quebra no
`vite build`, sem nada no diff explicando por quê.

O `knip` também acusou `@fontsource-variable/inter` e `@fontsource-variable/jetbrains-mono` como
dependências não usadas, e aí ele está errado: os dois são usados por `url()` dentro de `@font-face` em
`src/styles.css`, e o `knip` não segue `url()` de CSS.

## Solution

Declarar as três em `dependencies`, na faixa `^` da versão que o `bun.lock` **já resolve** — nenhuma
versão nova, nenhum pacote novo, nenhuma linha de código alterada. Manter os dois `@fontsource-variable/*`
e registrar a exceção do `knip` em `docs/decisions.md`, com o motivo, em vez de remover as fontes.

## User Stories

1. Como mantenedor, quero todo pacote importado pela UI declarado no `package.json`, para que o bump de
   um terceiro não possa removê-lo e quebrar o build em silêncio.
2. Como mantenedor, quero que a faixa declarada seja a versão que o `bun.lock` já resolve, para que
   declarar não mude o que está instalado e o PR não seja um bump disfarçado.
3. Como contribuidor, quero que `bun install --frozen-lockfile` continue passando, para que um clone
   novo e o CI instalem exatamente o que o lock diz.
4. Como contribuidor, quero que `bun run build` continue passando, para que busca, autocomplete e
   highlighting do editor continuem empacotando.
5. Como revisor, quero o motivo de **declarar** em vez de trocar o import escrito, para que a próxima
   pessoa não tente de novo o caminho dos reexports e perca o mesmo tempo.
6. Como revisor, quero o falso positivo do `@fontsource-variable/*` registrado com a razão, para que
   ninguém remova as fontes da UI acreditando no `knip`.
7. Como mantenedor, quero que o lock não ganhe nenhuma versão nova, para que a superfície da auditoria de
   dependências não cresça para resolver um problema de declaração.
8. Como mantenedor, quero que o PR seja classificado `risk:high` pelos caminhos que toca, para que uma
   mudança de dependência passe pelo mantenedor em vez de entrar sozinha.
9. Como usuário, quero que o editor continue encontrando, contando e indo para a linha depois da mudança,
   para que uma edição de manifesto não quebre comportamento.
10. Como sessão futura, quero a lacuna (nenhuma guarda de regressão foi criada) registrada no retro, para
    que ela não seja assumida como coberta.
11. Como mantenedor, quero que os outros achados do `knip` (8 exports e 7 tipos sem uso) fiquem
    explicitamente fora desta issue, para que o escopo não cresça de declaração para limpeza de API.

## Implementation Decisions

- **Declarar, não trocar o import.** `codemirror` não reexporta nada do que o editor usa: ele importa
  `searchKeymap`/`highlightSelectionMatches` e `autocompletion`/`closeBrackets`/`completionKeymap` só para
  montar o `basicSetup`, e não exporta nenhum deles. `tags`, `getSearchQuery`, `gotoLine`,
  `searchPanelOpen`, `insertCompletionText`, `pickedCompletion` e os tipos `Completion*` existem apenas
  nos pacotes originais. Trocar o import exigiria reescrever a lógica do editor e ainda assim declarar
  dois dos três.
- **Faixa `^` com a versão do lock**, como todo o resto do arquivo: `^1.2.5`, `^6.7.2`, `^6.20.4`. O lock
  continua fixando a resolução de hoje, então a mudança não instala nada diferente.
- **`dependencies`, não `devDependencies`**: os três entram no bundle da UI, como os `@codemirror/*` que
  já estão lá.
- **O `bun.lock` muda só na tabela de dependências do workspace raiz.** Nenhuma entrada nova em
  `packages`; o diff do lock é a prova de que não entrou versão nova.
- **O `@fontsource-variable/*` fica.** A exceção é registrada em `docs/decisions.md`, não em `knip.json`:
  o `knip` não é dependência do repositório, então uma config dele hoje é config morta de ferramenta que
  ninguém roda. A adoção do `knip` (com a config junto) é a frente adiada de `docs/SPEC-CI-PRACTICES.md`.
- **Nenhuma guarda de regressão nesta issue.** Um teste que varresse os imports de `src/` e comparasse com
  o `package.json` fecharia a classe do bug sem o `knip`, mas é um parser de import por regex para
  responder a mesma pergunta que o `knip` responde. A lacuna vai para o retro.
- **Tier de risco.** O PR toca `package.json` e `bun.lock`: pela tabela de `docs/SPEC-RISK-TIERS.md` isso
  é mudança de dependência, então `risk:high` e o merge é do mantenedor. Inevitável para a issue.

## Testing Decisions

- **O que faz um bom teste aqui**: provar (a) que manifesto e lock concordam, (b) que os três pacotes
  continuam resolvíveis no bundle e (c) que o comportamento que depende deles continua de pé — através de
  seams que já existem. Um teste novo que reimplemente a checagem do `knip` seria uma segunda resposta
  para a mesma pergunta, com um parser próprio para manter.
- **Seams, do mais alto para o mais baixo, todos existentes:**
  1. `bun install --frozen-lockfile` — `package.json` e `bun.lock` concordam (critério de aceite literal).
  2. `bun run build` (`tsc --noEmit && vite build`) — o `vite build` resolve os três no bundle. É a única
     prova que existe para `tags`/`@lezer/highlight`, porque nenhum spec asserta classe de token.
  3. `bun run check` — typecheck + `cargo check`, o que o job `typescript` do CI roda.
  4. Playwright em modo web, host real + fixture `OrderDesk`: `tests/e2e/editor-find-count.spec.ts`
     exercita `@codemirror/search` em runtime (painel `.cm-search`, contagem, replace all, Enter no próximo
     match) e `tests/e2e/medium-nav-navigation.spec.ts` cobre o `gotoLine` do mesmo pacote. Nenhum dos dois
     está quarentenado ou na lista do smoke.
- **Nenhum arquivo de teste novo**, nenhum `data-testid` novo.
- **Prior art**: os specs de editor em `tests/e2e/` já abrem a fixture e assertam o painel de busca; o job
  `typescript` do CI já roda `tsc --noEmit` e `bun test tests/unit`.
- **O `--frozen-lockfile` não é guarda.** Medido: no bun 1.3.9 (a versão pinada nos workflows) ele sai 0 com
  a tabela de dependências do workspace desatualizada, porque os pacotes já estão em `packages` por
  transitividade. O critério de aceite continua valendo (manifesto e lock concordam no fim), mas quem
  detecta a lacuna é o diff do lock, não o comando.

## Out of Scope

- Adotar o `knip` (dependência nova) ou criar `knip.json`; é a frente adiada de `docs/SPEC-CI-PRACTICES.md`.
- Os outros achados do `knip`: 8 exports e 7 tipos sem uso.
- Qualquer bump de versão, inclusive dos três pacotes declarados.
- Remover `@fontsource-variable/inter` ou `@fontsource-variable/jetbrains-mono`.
- A guarda de regressão de dependência não listada (registrada no retro).

## Further Notes

- Evidência: `docs/evidence/declared-deps.md`.
- Decisões: entrada de 2026-11-07 em `docs/decisions.md`; termo **Dependência não listada** no
  `GLOSSARY.md`.
- A issue-mãe é a #142; a medição do `knip` foi feita na base `39c4c17` e remedida em 2026-11-07 com os
  mesmos números.
