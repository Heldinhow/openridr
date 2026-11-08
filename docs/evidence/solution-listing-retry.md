# Evidência — nível de listagem que falha é repetido com limite (#282)

Issue: #282 (achado irmão de #264). Ambiente: macOS ARM64, worktree isolada, portas 1520/4517 (home
do E2E derivada da porta da UI). Data: 2026-11-07.

## Causa raiz

`createListingCache.ensure` (`src/solutionListing.ts`) tratava uma leitura que **não chegou** como
uma pasta **vazia**: o `catch` gravava `dirs[dir] = []` e o guarda de entrada (`if (!force &&
dirs[dir]) return`) aceitava esse array — vazio é *truthy*. Toda expansão seguinte devolvia o mesmo
vazio sem pedir de novo, até um `reload()` (outra revisão do Git, outro workspace, alternar "Show all
files") zerar a epoch.

## Política escolhida

A mesma das consultas de marcador da árvore (#264): a leitura original mais duas repetições, com
200 ms e 400 ms de espera (`LISTING_RETRY_DELAYS_MS`). Duas diferenças, ambas justificadas:

- **Depois do último fracasso o nível não é cacheado.** Com o retry, o nível só fica vazio quando a
  falha é definitiva; gravar `[]` ali faria a pasta continuar vazia até o próximo `reload()` — o
  sintoma que a issue descreve. Não cachear faz a próxima expansão perguntar de novo, e o nível não
  aparece como "pasta sem filhos" no cache.
- **A repetição para quando a epoch mudou.** Um pedido substituído por `reload()` não tem onde
  aterrissar, então não gasta as duas tentativas restantes nem pinta a árvore nova.

O resto do contrato do cache (epoch, `pending` compartilhando um pedido por nível, `snapshot`) ficou
como estava. Nenhuma mudança no host, nenhum `data-testid` tocado, nenhuma dependência nova.

## Falha-antes medida

- Unitário (política): com `src/solutionListing.ts` de `806a51a`, o teste novo
  `a level that never answers is asked three times and is not cached as an empty folder` falha em
  `expect(calls).toBe(3)` (recebe 1: não há repetição) e `expect(cache.snapshot()[""]).toBeUndefined()`
  (recebe `[]`).
- E2E: com o mesmo arquivo de `806a51a` e o spec novo, `bunx playwright test
  tests/e2e/solution-explorer.spec.ts -g "a directory level that fails to list"` → **1 failed**,
  `getByTestId('tree-file-docs/notes.md')` não aparece em 10 s (a pasta fica vazia). Com o conserto:
  **1 passed** em 2,8 s.

## A injeção do E2E é no nível que se expande, não na primeira chamada da página

O critério da issue pede o spec falhando a **primeira** chamada de `workspace.tree`. Medido: falhar a
primeira chamada da página (o nível raiz, `dir: ""`) **passa mesmo com o bug** — o clique em "Show
all files" dispara o `reload()` do cache, que relê todos os níveis abertos, e é exatamente a escotilha
de escape que a issue descreve ("até algo alheio forçar um reload"). Por isso o spec falha a primeira
chamada **do nível que é expandido** (`docs`), que é o nível cuja falha a issue descreve e o único em
que a falha-antes aparece: medido 1 failed com o `solutionListing.ts` de `806a51a` e 1 passed com o
conserto. A diferença fica registrada aqui e na issue.

## Bateria

- `bun test tests/unit` → 127 passed, 0 fail (17 arquivos).
- `bunx playwright test tests/e2e/solution-explorer.spec.ts` → 13 passed (1,3 min), incluindo o spec
  novo.
- `bunx playwright test tests/e2e/solution-git-status.spec.ts` → 3 passed (o outro spec que usa o
  mesmo cache: `.gitignore`, symlink e status).
- `bunx tsc --noEmit` limpo.

## Não verificado

- O app Tauri real: mudança de UI pura (cache de listagem), nada de shell nativo — a superfície da
  decisão de 2026-11-07 é o Playwright em modo web.
- Windows e macOS Intel.
- A taxa do sintoma em produção (a issue não trouxe medição; o desenho foi lido no código).
- Repetição com `--repeat-each`: o spec novo falha de forma determinística (a primeira chamada é
  injetada por `page.route`), então não há taxa a medir.

## Ambiente

O setup do E2E (`tests/e2e/environment.ts`, `ensureRealCsharpLs`) instala o `csharp-ls` 0.28.0 fixado,
que é um pacote **net10.0**: exige um SDK 10.x. Esta máquina só tinha o SDK 8.0.408 em `~/.dotnet`,
então a run usou um SDK 10.0.403 efêmero em `/tmp/openridr-dotnet10` no `PATH` (`DOTNET_ROOT` junto),
sem tocar em `~/.dotnet` — o `global.json` das fixtures continua resolvendo o 8.0.408. `PLAYWRIGHT_CHANNEL=chromium`
porque esta máquina não tem o Chrome do canal default.
