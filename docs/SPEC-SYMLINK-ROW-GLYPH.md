# SPEC — Glifo único na linha de symlink (smoke noturno verde)

Issue-mãe: [#245](https://github.com/Heldinhow/openridr/issues/245) — "e2e: solution-git-status symlink spec keeps the nightly smoke red".
Reafirma a decisão de UI do [#49](https://github.com/Heldinhow/openridr/issues/49) (`SPEC-SOLUTION-GIT-STATUS`, "Symlink na UI: marcado e não seguido").

## Problem Statement

O smoke noturno está vermelho em `main` desde que `solution-git-status.spec.ts` entrou na lista: o teste
"symlinks are marked and a symlinked folder is not expanded" falha no runner de macOS com
`getByTestId('tree-file-LinkDir').locator('.icon-link')` resolvendo **2** elementos onde o teste espera
1. A falha é determinística e reproduz localmente — não é ambiente do runner.

Para quem usa o app, o efeito é a linha de uma pasta symlink mostrar **dois glifos de corrente
sobrepostos**: o glifo da linha e o badge de canto desenhado por cima dele. O desenho aprovado no #49
diz outra coisa: na listagem a linha do link é uma folha **com glifo de corrente**; o badge de canto
existe para o link que aparece como fonte avaliada, que mantém o glifo de arquivo e ganha o marcador
no canto. O produto desvia do próprio desenho desde o commit do #49.

O custo do vermelho é o de sempre: um smoke que falha por spec desatualizado esconde a regressão de
verdade da noite seguinte, e o teste que deveria provar o marcador de symlink não prova nada.

## Solution

Cada linha de symlink passa a mostrar **um** glifo de corrente, no lugar certo: como ícone da linha
quando a linha não tem ícone próprio (listagem "Show all files" e pasta symlink no modo normal), e
como badge de canto quando a linha já tem ícone próprio (arquivo de fonte avaliada, que mantém o
glifo de arquivo). O restante do contrato da linha não muda: `data-symlink="true"`, descrição AX
"Symbolic link — not followed", nenhum chevron e nenhuma expansão.

O spec E2E que já existe passa a travar o desenho explicitamente (um glifo de corrente, zero badges
de canto na linha do link), e o smoke noturno volta a verde por um `workflow_dispatch` na branch.

## User Stories

1. Como desenvolvedor, quero ver um único glifo de corrente na linha de uma pasta symlink, para não ler o marcador como defeito de renderização.
2. Como desenvolvedor, quero que a linha de um symlink que é fonte avaliada continue mostrando o glifo de arquivo com o badge de corrente no canto, para reconhecer o tipo do arquivo e o link ao mesmo tempo.
3. Como desenvolvedor, quero que a linha de uma pasta symlink continue sem chevron, para não tentar expandir algo que a árvore não segue.
4. Como desenvolvedor, quero que clicar numa pasta symlink não liste filhos, para a árvore nunca seguir um alvo desconhecido.
5. Como usuário de leitor de tela, quero continuar ouvindo "Symbolic link — not followed" na descrição da linha, para saber que é link sem depender do glifo.
6. Como usuário de leitor de tela, quero que a linha continue sendo um único alvo focável, para não ganhar uma parada de tabulação a mais.
7. Como usuário em modo restrito, quero ver o glifo de corrente mesmo sem confiança, para saber o que estou olhando sem executar nada.
8. Como mantenedor, quero o spec E2E afirmando o desenho (um glifo, sem badge de canto), para a regra não regredir silenciosamente na próxima mexida na árvore.
9. Como mantenedor, quero o smoke noturno verde, para a falha da próxima noite ser sinal de regressão e não ruído conhecido.
10. Como mantenedor, quero a causa raiz escrita com o run e a reprodução local, para ninguém reabrir a investigação do zero.
11. Como mantenedor, quero que a correção não acrescente prop, dependência nem seam novo, para a árvore continuar com uma regra só de marcador.

## Implementation Decisions

- **Uma regra no `TreeRow`, sem prop nova.** A linha desenha o glifo de corrente como ícone quando é
  `symlink` e não recebe ícone próprio; desenha o badge de canto quando é `symlink` **e** recebe um
  ícone próprio. O `symlink` continua sendo a única entrada: `data-symlink`, descrição AX e ausência
  de chevron não mudam.
- **Dois call sites deixam de passar o ícone de link à mão.** A listagem "Show all files" e a pasta
  symlink do modo normal passam a confiar na regra do `TreeRow`; o arquivo de fonte avaliada continua
  passando o glifo de arquivo e por isso continua ganhando o badge.
- **Nada muda no host.** `workspace.tree` e `workspace.symlinks` ficam como estão; o desvio é só de
  apresentação.
- **O contrato de teste da linha é o mesmo do #49**, endurecido: `data-symlink="true"`, um
  `.icon-link` na linha do link de pasta e nenhum `.symlink-mark` nela; a linha do arquivo symlink
  avaliado mantém um `.symlink-mark`.
- **Sem dependência nova, sem `data-testid` novo.** O glifo continua vindo do conjunto Lucide já
  instalado.
- **Sem quarentena e sem issue filha.** A causa raiz é um desvio de três linhas do desenho aprovado,
  medido e determinístico; quarentenar aqui esconderia um defeito visível.

## Testing Decisions

- **O que é um bom teste aqui:** ele observa o que o usuário vê — quantos glifos de corrente a linha
  tem, se a linha tem chevron, se clicar nela lista filhos — e não a estrutura interna que decide
  isso. Nada de testar o `TreeRow` por dentro.
- **Seam único: o DOM da linha da árvore**, pelo spec E2E que já existe
  (`tests/e2e/solution-git-status.spec.ts`), com repositório descartável criado pelo teste. É o mesmo
  seam do #49; nenhum renderer de teste novo (o repo não tem um) e nenhum seam de host.
- **Falha-antes/passa-depois:** o assert atual já falha hoje de forma determinística (medido: 2
  elementos onde se espera 1), então o teste é o próprio vermelho-verde da correção.
- **Endurecimento de uma linha:** além de `.icon-link` = 1, a linha do link de pasta afirma
  `.symlink-mark` = 0, para o desenho ficar explícito em vez de depender de uma contagem que só
  fecha por acidente.
- **Verificação de UI:** Playwright em modo web, só este spec, nas portas da sessão; o smoke noturno
  é provado por `workflow_dispatch` do `e2e-smoke.yml` na branch, com a URL na evidência.
- **Sem app real:** nada aqui é específico do shell (menu nativo, atalho do sistema, seletor nativo,
  árvore de acessibilidade do WebKit, updater, DMG).

## Out of Scope

- Redesenho do badge de canto ou do glifo de corrente; qualquer mudança de cor, tamanho ou posição.
- Nó de projeto/pasta symlink fora dos dois casos citados, e agregar status Git em pasta.
- Quarentenar o spec ou abrir follow-up: a issue aceita quarentena, mas a causa é um desvio pequeno e
  visível.
- Os outros specs do smoke e a lista `scripts/e2e-smoke.txt` além do comentário que dizia "ainda NÃO
  medido".
- Qualquer mudança no host, no contrato de `workspace.tree`/`workspace.symlinks` ou no confinamento.

## Further Notes

Medição da causa (2026-11-07): run noturno
[42364384217](https://github.com/Heldinhow/openridr/actions/runs/42364384217) falha em
`solution-git-status.spec.ts:111` com `Locator: getByTestId('tree-file-LinkDir').locator('.icon-link')`,
`Expected: 1`, `Received: 2`. Reprodução local idêntica na branch da issue (portas 1820/4717):
2 passed, 1 failed, mesmo assert e mesmo `Received: 2`. O run noturno também mostra 27 passed, então
a falha é isolada a este teste.

O desenho não é novo: `SPEC-SOLUTION-GIT-STATUS` já decidiu "na listagem a linha de um link é uma
folha com glifo de corrente … um link que aparece como fonte avaliada mantém o glifo do arquivo e
ganha o badge de corrente no canto". Este spec só faz o produto cumprir o que o #49 decidiu e trava
o contrato no E2E.
