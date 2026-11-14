# Evidência — ícones Seti da Solution Explorer

Spec: [#353](https://github.com/Heldinhow/openridr/issues/353). Tickets:
[#354](https://github.com/Heldinhow/openridr/issues/354),
[#355](https://github.com/Heldinhow/openridr/issues/355),
[#356](https://github.com/Heldinhow/openridr/issues/356),
[#357](https://github.com/Heldinhow/openridr/issues/357),
[#358](https://github.com/Heldinhow/openridr/issues/358).

O estado anterior (glifos de três famílias, 29 cores, selo do `.cs`) está em
[`solution-file-icons.md`](solution-file-icons.md) e nas capturas de
[`solution-file-icons/`](solution-file-icons/).

## Como reproduzir

O spec de E2E tira as capturas quando `OPENRIDR_FILE_ICONS_EVIDENCE` aponta para uma pasta:

```sh
PLAYWRIGHT_CHANNEL=chromium OPENRIDR_FILE_ICONS_EVIDENCE=docs/evidence/solution-file-icons-seti \
  bunx playwright test tests/e2e/solution-file-icons.spec.ts
```

A fixture é uma **cópia temporária** de `fixtures/OrderDesk` (em `/tmp`) com `README.md`,
`appsettings.json`, `Dockerfile`, `docker-compose.yml`, `OrderDesk.sln.DotSettings.user`, `.idea/` e
`src/OrderDesk.Core/bin/` criados só na cópia. A fixture versionada não muda.

O app real foi rodado no Linux (WebKitGTK) a partir do binário desta worktree:

```sh
bun run build
bunx tauri build --debug --no-bundle --config '{"build":{"beforeBuildCommand":""}}'
OPENRIDR_HOME=/tmp/openridr-icons \
OPENRIDR_BUNDLED_NETCOREDBG=$PWD/src-tauri/resources/netcoredbg \
  src-tauri/target/debug/openridr /tmp/openridr-icons-work/OrderDesk/OrderDesk.sln
```

`OPENRIDR_HOME` isolado, `trusted.json` semeado com o caminho canônico da cópia (o diálogo de trust
não recebe clique nesta sessão), ferramenta `csharp-ls` copiada de `~/.local/share/openridr/tools`
para o home isolado, runtime .NET 10 do `~/.dotnet`, netcoredbg vendorizado em
`src-tauri/resources/netcoredbg`. A janela foi movida para o workspace 7 e capturada por PID.

## Depois

| Arquivo | O que mostra |
|---|---|
| [`depois-glyphs.png`](solution-file-icons-seti/depois-glyphs.png) / [`-arvore`](solution-file-icons-seti/depois-glyphs-arvore.png) | Show all files: Seti em `.cs` (hexágono azul), `.json` (`{}` amarelo), `.md` (`M↓` azul), `Dockerfile`/`docker-compose.yml` (baleia azul), `.DotSettings.user` (página cheia), `LICENSE` (página amarela); pasta aberta/fechada |
| [`depois-pastas-e-guia.png`](solution-file-icons-seti/depois-pastas-e-guia.png) | Pasta com dois desenhos (aberta/fechada), guias na coluna do chevron, nome de pasta alinhado ao de arquivo |
| [`depois-filtro-tipo.png`](solution-file-icons-seti/depois-filtro-tipo.png) | Filtro ativo: `Order.cs` com `class` como texto na linha, nome acessível intacto |
| [`depois-superficies.png`](solution-file-icons-seti/depois-superficies.png) | O mesmo `.cs` na árvore, na aba, no Find in Files e no Search Everywhere |
| [`depois-projeto-duas-camadas.png`](solution-file-icons-seti/depois-projeto-duas-camadas.png) | Linha de projeto com `console · net8.0` na segunda camada |
| [`depois-contraste.png`](solution-file-icons-seti/depois-contraste.png) | A linha medida pelo teste de contraste |
| [`desktop-01-arvore.png`](solution-file-icons-seti/desktop-01-arvore.png) / [`desktop-02-arvore-recorte.png`](solution-file-icons-seti/desktop-02-arvore-recorte.png) | App real (WebKitGTK): a árvore com os glifos Seti e as linhas de projeto em duas camadas |

## Asset

| Campo | Valor |
|---|---|
| Fonte | `microsoft/vscode`, tag `1.145.2`, `extensions/theme-seti/icons/seti.woff` |
| Tamanho | 37.284 bytes |
| SHA-256 | `b127762058f89b37b08d76185e3b0558f5c28859e39a3840a27cb28b9739d6e2` |
| Origem da fonte | `jesseweed/seti-ui`, commit `2d6c5e68b4ded73c92dac291845ee44e1182d511` |
| Licença | MIT (VS Code e `jesseweed/seti-ui`), aviso em `THIRD-PARTY-NOTICES.md` |

A tabela (`FILE_TYPES`, `FILE_NAMES`, `FILE_EXTENSIONS` em `src/solutionVisuals.ts`) é escrita à mão
a partir do `vs-seti-icon-theme.json` da mesma tag; o JSON (54 KB) não entra no bundle. Os
codepoints ficam em `src/FileIcon.tsx`. O `seti.woff` é o arquivo upstream, sem modificação.

## Métrica do glifo em 16 px

O glifo é um caractere da fonte a 20 px (o `150%` de um rótulo de 13 px, como no VS Code) dentro do
slot de 16 px. Medido por pixel (tinta = pixels diferentes do fundo):

| Plataforma | Glifo | Caixa da tinta | Tinta / caixa | Centro no slot |
|---|---|---|---|---|
| Chromium (1,6×) | `_c-sharp` | 17×16 px de device (10,6×10 lógicos) | 0,39 | sim (4 px de folga em cada lado) |
| WebKitGTK (1,6×, app real) | `_c-sharp` | 16×15 px de device (10×9,4 lógicos) | 0,35 | sim |

A caixa e a densidade batem entre os dois motores; a diferença é de rasterização. O glifo não é
tofu: a tinta é o hexágono preenchido, com a mesma proporção do desenho de referência.

## Paleta (10 tokens com cor, estrutura em `--muted`)

Piso de 3:1 (WCAG 1.4.11) em todas as superfícies de linha; pior caso em negrito.

| token | valor | panel | hover | raised | select | focada | no index | no index hover |
|---|---|---|---|---|---|---|---|---|
| `--icon-seti-blue` | `#519aba` | 6,05 | 5,54 | 5,48 | 4,69 | **3,96** | 5,35 | 4,94 |
| `--icon-seti-yellow` | `#cbcb41` | 11,02 | 10,08 | 9,97 | 8,54 | 7,21 | 9,74 | 8,99 |
| `--icon-seti-purple` | `#a074c4` | 5,25 | 4,80 | 4,75 | 4,06 | **3,43** | 4,64 | 4,28 |
| `--icon-seti-green` | `#8dc149` | 8,93 | 8,17 | 8,08 | 6,92 | 5,84 | 7,90 | 7,29 |
| `--icon-seti-orange` | `#e37933` | 6,39 | 5,84 | 5,78 | 4,95 | 4,18 | 5,65 | 5,21 |
| `--icon-git` | `#708691` | 4,99 | 4,56 | 4,52 | 3,87 | **3,27** | 4,41 | 4,07 |
| `--icon-project-console` | `#7cc0ff` | 9,82 | 8,98 | 8,89 | 7,61 | 6,43 | 8,69 | 8,01 |
| `--icon-project-library` | `#8ed39a` | 10,81 | 9,89 | 9,79 | 8,38 | 7,08 | 9,57 | 8,82 |
| `--icon-project-test` | `#e0b76e` | 10,12 | 9,26 | 9,16 | 7,84 | 6,62 | 8,95 | 8,26 |
| `--icon-project-web` | `#7fd3e8` | 11,22 | 10,26 | 10,16 | 8,70 | 7,34 | 9,93 | 9,16 |

`--icon-git` sobe de `#41535b` (Seti, 1,55:1 na linha focada) para `#708691` (3,27:1), o slate mais
próximo que passa. Pasta, `Dependencies`, `Frameworks`, `Packages`, `.sln`/`.resx`/`.editorconfig`/
`.DotSettings` e o genérico usam `--muted` (5,94:1 na pior superfície) e não têm token próprio.
`tests/unit/iconContrast.test.ts` lê o `src/styles.css` e o `src/solution-icons.css` reais e falha se
qualquer token colorido cair abaixo de 3:1 em qualquer superfície, ou se a paleta passar de 10
tokens.

## Resolvedor

Ordem do VS Code: **nome** > **sufixo mais longo** > **extensão** > genérico. Casos que o teste
unitário fixa:

| caminho | tipo | por quê |
|---|---|---|
| `global.json` | json | nome |
| `Dockerfile.dev` | docker | prefixo de nome |
| `docker-compose.override.yml` | docker | sufixo mais longo vence `yml` |
| `deploy/pipeline.yml` | yaml | extensão |
| `OrderDesk.sln.DotSettings.user` | dotsettings | sufixo `dotsettings.user` vence `user` |
| `Settings.user` | file | genérico |
| `LICENSE` | license | nome |
| `tool.exe` | file | genérico |

## Testes

- `bun test tests/unit` — **169 passam**. Inclui `solutionVisuals` (resolvedor, glifo, token,
  `projectMeta`), `iconContrast` (piso e teto da paleta) e `treeClasses` (toda classe da árvore tem
  regra de CSS).
- `bunx playwright test tests/e2e/solution-file-icons.spec.ts` — **11 passam**: glifo por linha,
  pasta aberta/fechada, contraste medido no CSS carregado, cabeçalho sem chevron, Dependencies/
  Frameworks com glifo próprio, filtro com o tipo em texto, o mesmo glifo em quatro superfícies,
  segunda camada do projeto e recuo de 16 px, seleção arredondada.
- `solution-explorer.spec.ts`, `medium-nav-search.spec.ts`, `solution-git-status.spec.ts` — verdes.
- `qa4-eval-failed.spec.ts` — verde, com a asserção nova de prioridade da tag: numa linha de projeto
  com avaliação falha, `.tag.error` tem `flex-shrink: 0` e `.tree-name` tem `flex-shrink: 1`, então o
  nome é o que encolhe primeiro.
- `settings.spec.ts` — 2 falhas de ambiente (atalho `⌘,` e rótulo `⇧⌘F9` são do macOS; o host reporta
  `linux`, então o `primary` é Ctrl), não do diff. O mesmo vale para 3 falhas de
  `medium-nav-find-in-files.spec.ts` (`Meta+Shift+F`).
- `bun run check` — tsc limpo; `cargo check --workspace --all-targets` limpo.

## Não verificado

- **macOS/WKWebView**: a métrica do glifo foi medida no WebKitGTK e no Chromium; o alvo verificado do
  produto (macOS) não foi executado nesta sessão.
- **Windows e macOS Intel**: continuam não verificados.
- **Git panel e Search Everywhere no app real**: provados no Playwright (Chromium), não no WebKitGTK.
- **Densidade**: o AC da #358 fala em 22 px, mas o `rider-chrome.css` fixa 36 px (confortável) e 32 px
  (compacta) desde a #351, e `settings.spec.ts` testa exatamente isso. A densidade **não** foi
  alterada; a linha de projeto é a linha de duas camadas e cabe nos 36 px. Alinhar o código ao 22 px
  quebraria a densidade testada e o redesenho aprovado.
