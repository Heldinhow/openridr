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
| [`depois-contraste.png`](solution-file-icons/depois-contraste.png) / [`depois-contraste-arvore.png`](solution-file-icons/depois-contraste-arvore.png) | `OrderDesk.sln.DotSettings.user` com o corpo do documento mais forte (a captura do teste que mede o contraste) |

## Contraste dos glifos de dois tons (WCAG 1.4.11)

O corpo da página (`.glyph-page`) usa `opacity` sobre o token cheio de documento/texto/baleia. A
primeira versão usava `.5` e reprovava:

| Fundo da linha | `.5` (antes) | `.75` (agora) |
|---|---|---|
| `--panel` `#0b0b0b` | **2,95:1** | 5,31:1 |
| `--raised` `#141414` | 2,96:1 | 5,13:1 |
| hover (`--accent-surface` sobre `--panel` ≈ `#161616`) | 2,95:1 | 5,07:1 |
| linha `no index` (âmbar 9% sobre `--panel` ≈ `#1f1a10`) | 2,92:1 | 4,92:1 |
| linha atual focada (`--select` `#1e3358`) | 2,52:1 | **3,86:1** |

O pior caso é a linha atual focada, porque `--select` é um azul médio e o corpo do glifo é misturado com
ele; `.75` mantém 3,86:1 ali (`.7` daria 3,56:1, com menos folga). O glifo de um tom só (json, markdown,
yml, git) usa o token cheio e não muda.

Tema claro (não distribuído, **não verificado** no app): a transparência não serve — `#84848a` a `.75`
sobre branco dá 2,34:1. Por isso `prefers-color-scheme: light` dentro do `@supports` dos tokens leva o
corpo a `opacity: 1`: 3,72:1 (documento) e 3,93:1 (baleia) sobre `#fff`. Se o tema claro for
habilitado, esse é o valor a manter.

Medição automatizada: `tests/e2e/solution-file-icons.spec.ts` lê `--icon-doc`, `--panel`, `--raised`,
`--select` e a `opacity` computada do `.glyph-page` do CSS carregado e exige ≥ 3:1 nos três fundos.

## Medido nos testes
- `tests/unit/solutionVisuals.test.ts` — 28 casos verdes no total da pasta `tests/unit`; cobre
  nome/extensão → kind → glifo.
- `tests/e2e/solution-file-icons.spec.ts` — 5/5 nas portas 1490/4387: `[data-icon]` por arquivo, pasta com
  `.tree-icon` vazio, `tree-guide` por nível (raiz 0, filho 1, neto 2), `box-shadow` da linha atual
  `inset … 84, 138, 247` e altura < 36 px, descrição acessível "C# class" com o nome acessível intacto, e o
  piso de 3:1 do corpo dos glifos medido no CSS real (tabela acima).
- Regressão de árvore/layout: `solution-explorer`, `native-context-menu`, `ui-layout`, `medium-nav-layout`,
  `workspace` — 41/42; a única falha (`workspace.spec.ts:94`, texto de `sdk-debug-adapter`) é o app home
  compartilhado de E2E com `bundled/netcoredbg` semeado por uma sessão anterior (diretório de 09:55, antes
  desta sessão), não o diff de ícones.

## App real (agent-desktop, Tauri/WKWebView)

Build **desta branch**: `bun run tauri build --debug --bundles app` →
`src-tauri/target/debug/bundle/macos/OpenRidr.app` (0.3.0-alpha.7, `doctor.sh` ok). Sessão isolada:
`desktop.sh up` com `OPENRIDR_HOME=<tmp>/verify-openridr-<uid>-desktop/home`, cópia da fixture em
`<tmp>/verify-openridr-<uid>-desktop/work/OrderDesk` + `Dockerfile`, `docker-compose.yml`,
`appsettings.json`, `README.md`, `.gitignore` e `Services/OrderService.cs` criados **só na cópia**. Trust:
"Open in Restricted mode" (o suficiente para a árvore; nada de build/restore).

| Arquivo | O que mostra |
|---|---|
| [`desktop-01-show-all-pastas-e-guia.png`](solution-file-icons/desktop-01-show-all-pastas-e-guia.png) | Janela real com Show all files: `Entities`/`Services` só com chevron, guias verticais, `.gitignore` com ramo, `{}` laranja, baleia azul nos dois Docker, `M↓` verde |
| [`desktop-02-glifos-recorte.png`](solution-file-icons/desktop-02-glifos-recorte.png) | Recorte da árvore: hexágono `C#` com selo de tipo em `Program.cs`/`Order.cs`/`OrderTests.cs`/`OrderService.cs`; `Services` aberta com guia até o filho |
| [`desktop-03-selecao-hexagono.png`](solution-file-icons/desktop-03-selecao-hexagono.png) | `Program.cs` clicado: contorno azul arredondado na linha, arquivo aberto no editor, hexágono na aba, breadcrumb e status bar |
| [`desktop-04-hexagono-selo-recorte.png`](solution-file-icons/desktop-04-hexagono-selo-recorte.png) | Recorte do hexágono + selo de 9 px e da linha selecionada |

O app foi fechado com `desktop.sh down` (`stopped`); depois, `pgrep -x openridr` vazio. Os `dotnet` que
seguem vivos na máquina são de outro teste (`/tmp/ncdbg-smoke`), com ~1 dia de uptime — não desta sessão.

**Não verificado no app real:** a descrição acessível do tipo de símbolo. O WKWebView expõe o conteúdo web
na árvore de acessibilidade (as linhas aparecem como `button: Program.cs`), mas **não** expõe o
`aria-describedby` — `agent-desktop snapshot` devolve `description: None` para `Program.cs`/`Order.cs`. A
descrição "C# class" só tem prova no Chromium (Playwright). Foco por teclado segue não dirigível nesta
superfície.

## Não verificado

- **App Tauri real (agent-desktop):** coberto acima para glifos, pastas, guias, seleção e Show all files.
  Continuam sem prova no app real a descrição acessível do símbolo (WKWebView) e o tema claro.
- **Tema claro:** não é distribuído (Settings só oferece o escuro); os pares `light-dark()` novos não foram
  vistos renderizados.
- **`.csproj`/`.sln` como linha de arquivo:** o Show all files esconde o `.csproj` do próprio projeto e o
  `.sln` da raiz (`skip` do `SolutionTree`), então esse glifo só tem prova unitária.
- **Letras do selo de tipo em 9 px:** legíveis nas capturas do app real (`desktop-02`/`desktop-04`), mas o
  nome do tipo só é garantido por `title`, `data-symbol-kind` e `aria-describedby`; a descrição não aparece
  na árvore de acessibilidade do WKWebView, então ela tem prova só no Chromium (Playwright).
