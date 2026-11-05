# Evidência — ícones da Solution Explorer

Spec: [SPEC-SOLUTION-FILE-ICONS](../SPEC-SOLUTION-FILE-ICONS.md). Tickets:
[01](../tickets-solution-file-icons/01-glifos-e-mapeamento.md), [02](../tickets-solution-file-icons/02-linhas-guia-selecao.md),
[03](../tickets-solution-file-icons/03-testes-e-evidencia.md).

## Como reproduzir

O spec de E2E tira as capturas quando `OPENRIDR_FILE_ICONS_EVIDENCE` aponta para uma pasta:

```sh
PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$HOME/.bun/bin:$PATH DOTNET_ROOT=$HOME/.dotnet \
  DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_UI_PORT=1490 OPENRIDR_E2E_HOST_PORT=4387 \
  OPENRIDR_FILE_ICONS_EVIDENCE=docs/evidence/solution-file-icons \
  bunx playwright test tests/e2e/solution-file-icons.spec.ts
```

A fixture usada é uma **cópia temporária** de `fixtures/OrderDesk` (no `/tmp`) com `README.md`,
`appsettings.json`, `Dockerfile`, `docker-compose.yml`, `OrderDesk.sln.DotSettings.user`, `.idea/` e
`src/OrderDesk.Core/bin/` criados só na cópia. A fixture versionada não muda. Tema escuro, 1440×900,
`channel: chrome`.

## Antes

O estado de `main@60e784f` (mesma árvore, wave anterior desta feature) está documentado em
[`solution-explorer-visual/01-tree-evaluated.png`](solution-explorer-visual/01-tree-evaluated.png) e
[`solution-explorer-visual/05-show-all-no-index.png`](solution-explorer-visual/05-show-all-no-index.png),
capturados no app real: toda pasta com ícone de pasta, `.cs` com o glifo `C#` de traço ou com o badge de
letra **no lugar** do ícone, `.sln`/`.csproj` com `Blocks`/`Box`, `.md` azul, `json` amarelo, sem guia de
indentação e seleção como bloco preenchido.

Não foi gerada uma captura "antes" nova nesta branch porque isso exigiria um segundo build do host; a
referência acima é a mesma tela no commit base.

## Depois

| Arquivo | O que mostra |
|---|---|
| [`depois-glyphs.png`](solution-file-icons/depois-glyphs.png) / [`depois-glyphs-arvore.png`](solution-file-icons/depois-glyphs-arvore.png) | Show all files: `{}` laranja em `appsettings.json`/`global.json`, baleia azul em `Dockerfile`/`docker-compose.yml`, `M↓` verde em `README.md`, documento cinza em `OrderDesk.sln.DotSettings.user`, ramo git em `.gitignore`, hexágono roxo com selo de tipo em `Program.cs`/`Order.cs`/`OrderTests.cs` |
| [`depois-pastas-e-guia.png`](solution-file-icons/depois-pastas-e-guia.png) / [`depois-pastas-e-guia-arvore.png`](solution-file-icons/depois-pastas-e-guia-arvore.png) | `bin` sem ícone (só chevron + nome), guias verticais nas colunas dos ancestrais, nome de pasta alinhado ao de arquivo |
| [`depois-selecao.png`](solution-file-icons/depois-selecao.png) / [`depois-selecao-arvore.png`](solution-file-icons/depois-selecao-arvore.png) | Linha atual (`Program.cs`) com contorno azul arredondado de 1 px, mesma altura das outras |

## Medido nos testes

- `tests/unit/solutionVisuals.test.ts` — 28 casos verdes no total da pasta `tests/unit`; cobre
  nome/extensão → kind → glifo.
- `tests/e2e/solution-file-icons.spec.ts` — 3/3 nas portas 1490/4387: `[data-icon]` por arquivo, pasta com
  `.tree-icon` vazio, `tree-guide` por nível (raiz 0, filho 1, neto 2), `box-shadow` da linha atual
  `inset … 84, 138, 247` e altura < 36 px.
- Regressão de árvore/layout: `solution-explorer`, `native-context-menu`, `ui-layout`, `medium-nav-layout`,
  `workspace` — 41/42; a única falha (`workspace.spec.ts:94`, texto de `sdk-debug-adapter`) é o app home
  compartilhado de E2E com `bundled/netcoredbg` semeado por uma sessão anterior (diretório de 09:55, antes
  desta sessão), não o diff de ícones.

## Não verificado

- **App Tauri real (agent-desktop):** outra sessão estava usando o app instalado; a captura no app fica
  pendente de liberação do orquestrador.
- **Tema claro:** não é distribuído (Settings só oferece o escuro); os pares `light-dark()` novos não foram
  vistos renderizados.
- **`.csproj`/`.sln` como linha de arquivo:** o Show all files esconde o `.csproj` do próprio projeto e o
  `.sln` da raiz (`skip` do `SolutionTree`), então esse glifo só tem prova unitária.
- **Letras do selo de tipo em 9 px:** legíveis na captura, mas o nome do tipo só é garantido por `title` +
  `data-symbol-kind`; não houve teste de leitor de tela no app real.
