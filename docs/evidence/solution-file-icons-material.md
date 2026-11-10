# Ícones da Solution Explorer: Material Icon Theme no lugar do Seti (#360)

O Seti do #359 saiu inteiro — fonte, tabela derivada, glifos do `@pierre/trees` e tokens `--icon-*`.
Os desenhos de arquivo **e de pasta** passam a vir do **Material Icon Theme** (MIT), copiados
verbatim de uma tag fixa, e a árvore desenha SVG em vez de caractere de fonte.

## Asset

| Campo | Valor |
|---|---|
| Repositório | `material-extensions/vscode-material-icon-theme` (antes `PKief/vscode-material-icon-theme`) |
| Tag / commit | `v5.40.0` → `045291ff033062a25441dd297fd51546326866f2` (2026-10-26) |
| Licença | MIT — `LICENSE` no commit: "The MIT License (MIT) / Copyright (c) 2025 Material Extensions" |
| Arquivos | 30 SVGs em `public/file-icons/`, 30 em `public/folder-icons/` (15 nomes × fechada/aberta) |
| Tamanho | 39.917 bytes no total (a fonte Seti sozinha eram 37.284) |
| Modificação | nenhuma: byte a byte o arquivo do tema, `fill` incluso |

As variantes abertas não existem no repositório do tema: ele as gera no build com
`src/scripts/svg/generateOpenFolderIcons.ts`, que troca o `d` do `#folder` pelo caminho canônico
aberto. As 30 abertas foram produzidas rodando **esse script** no commit fixado, não desenhadas aqui.

O mapeamento foi derivado do `material-icons.json` que o `generateJson` do próprio tema produz a
partir de `src/core/icons/fileIcons.ts` e `folderIcons.ts` no mesmo commit, com a precedência do
VS Code (nome inteiro > extensão mais longa > genérico).

## O que mudou no código

| Antes (Seti) | Depois (Material Icon Theme) |
|---|---|
| `src/assets/seti.woff` + `@font-face` | `public/file-icons/*.svg` + `public/folder-icons/*.svg` |
| `SETI` (codepoints) em `FileIcon.tsx` | `fileIcon(path)` → nome do SVG, em `solutionVisuals.ts` |
| `FILE_TYPES` (kind → glifo + token) | `KIND_ICON` + `ICON_NAMES`/`ICON_EXTENSIONS` (as exceções do tema) |
| `--icon-seti-*`, `--icon-git`, `--icon-project-*` | nenhum token de ícone; o `fill` é o do SVG |
| `FolderIcon` desenhado aqui, sem nome | `folderIcon(name)` → SVG do tema, aberto e fechado |
| `DependencyGlyph`/`RuntimeGlyph` desenhados aqui | `NugetGlyph` = o SVG do NuGet |
| `ProjectIcon` = Lucide por tipo de projeto | `ProjectIcon` = o SVG do `.csproj` (o mesmo do `.sln`) |
| `fileGlyphs.tsx` (`@pierre/trees`, Apache-2.0) | removido; o tema tem desenho para todos os casos |

`fileKind` **não mudou**: `data-file-kind` mantém os mesmos valores. O desenho e o tipo passaram a ser
duas resoluções separadas — o tipo é semântico (`.sln` ≠ `.csproj`), o desenho é o do tema (os dois
usam o mesmo SVG).

## Tabela: nó → SVG

| Nó | Ícone |
|---|---|
| raiz da solução (`.sln`, `.slnx`) | `visualstudio.svg` |
| projeto (`.csproj`) | `visualstudio.svg` |
| `Dependencies`, pacotes, `Frameworks` | `nuget.svg` |

## Tabela: arquivo → SVG

| Caso | Ícone |
|---|---|
| `.cs`, `.csx` | `csharp` |
| `.csproj`, `.sln`, `.slnx` | `visualstudio` |
| `.fsproj` | `fsharp` |
| `.vbproj`, `.xml`, `.targets`, `.resx` | `xml` |
| `.props`, `.config`, `*.sln.DotSettings`, `*.sln.DotSettings.user` | `settings` |
| `.razor`, `.cshtml` | `razor` |
| `.json`, `.jsonc`, `appsettings*.json`, `global.json`, `launchSettings.json` | `json` |
| `package.json` | `nodejs` |
| `tsconfig.json` | `tsconfig` |
| `nuget.config` | `nuget` |
| `.md`, `.markdown` | `markdown` |
| `README.md` | `readme` |
| `.mdx` | `mdx` |
| `.yml`, `.yaml` | `yaml` |
| `.editorconfig` | `editorconfig` |
| `.gitignore`, `.gitattributes`, `.gitmodules` | `git` |
| `Dockerfile`, `docker-compose*.yml`, `.dockerignore` | `docker` |
| `.sh`, `.bash`, `.zsh`, `.bat`, `.cmd` | `console` |
| `.ps1`, `.psm1` | `powershell` |
| `.txt` | `document` |
| `.svg` | `svg` |
| `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.ico` | `image` |
| `.css` | `css` |
| `.scss` | `sass` |
| `.less` | `less` |
| `.html`, `.htm` | `html` |
| `.xaml` | `xaml` |
| `.env` | `tune` |
| `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` | `license` |
| genérico | `file` |

## Tabela: pasta → SVG (fechada / aberta)

| Pasta | Fechada | Aberta |
|---|---|---|
| genérica | `folder` | `folder-open` |
| `src` | `folder-src` | `folder-src-open` |
| `test`, `tests` | `folder-test` | `folder-test-open` |
| `docs` | `folder-docs` | `folder-docs-open` |
| `Properties` | `folder-config` | `folder-config-open` |
| `wwwroot` | `folder-public` | `folder-public-open` |
| `bin` | `folder-dist` | `folder-dist-open` |
| `.github` | `folder-github` | `folder-github-open` |
| `.vscode` | `folder-vscode` | `folder-vscode-open` |
| `scripts` | `folder-scripts` | `folder-scripts-open` |
| `assets` | `folder-resource` | `folder-resource-open` |
| `Controllers`, `Services` | `folder-controller` | `folder-controller-open` |
| `Models` | `folder-class` | `folder-class-open` |
| `Views` | `folder-views` | `folder-views-open` |
| `Migrations` | `folder-migrations` | `folder-migrations-open` |

## O que o tema não mapeia (cai no genérico)

- pasta **`obj`** — não está em `folderNames`; usa `folder`/`folder-open`.
- **`.DotSettings` sozinho** (`App.DotSettings`) — o tema só conhece `sln.dotsettings` e
  `sln.dotsettings.user`; cai em `file`.
- **`.resources`** — extensão desconhecida; cai em `file`.

## Contraste

Medido nos `fill` dos 60 SVGs contra `--panel` (`#101010`), a superfície de repouso da linha:
**mínimo 3,52:1**, todos passam o piso de 3:1 de gráfico não textual (WCAG 1.4.11).

Na linha focada (`--select-row-focus`, `#343434`) seis desenhos ficam abaixo do piso — e a decisão é
**manter a cor do tema** em vez de inventar uma variante clareada:

| Desenho | `fill` | Linha focada |
|---|---|---|
| `visualstudio` (`.sln`, `.csproj`) | `#ab47bc` | 2,58:1 |
| `css` | `#7e57c2` | 2,39:1 |
| `less` | `#0277bd` | 2,59:1 |
| `tsconfig` | `#757575` | 2,70:1 |
| `folder-github`, `folder-scripts` (corpo) | `#546e7a` | 2,31:1 |

O `iconContrast.test.ts` lê os SVGs e o CSS e afirma os dois lados: o piso na superfície de repouso e
a lista acima como dado, não como silêncio.

## Verificação

### Unit (`bun test tests/unit`)

`171 pass, 0 fail`. Os testes tocados:

- `solutionVisuals.test.ts` — `fileIcon` cobre **cada caso** das tabelas acima (70 pares nome → SVG),
  a precedência nome > sufixo mais longo e os três "não mapeados"; `folderIcon` cobre os 16 nomes
  e o genérico.
- `iconContrast.test.ts` — todo nome que o resolvedor pode devolver tem SVG (e nada é enviado sem
  uso), todo `fill` passa 3:1 em `--panel`, a lista da linha focada é a esperada, e não sobrou
  `seti` nem `--icon-` no CSS.
- `treeClasses.test.ts` — nenhuma classe sem regra; `tint-` e `project-<kind>` não existem mais.

### E2E (`tests/e2e/solution-file-icons.spec.ts`, Playwright em modo web)

`11 passed`. Cobre: `.sln`/projeto com `data-icon="visualstudio"`, `Dependencies`/pacotes/
`Frameworks` com `nuget`, `.cs`/`.json`/`.md`/`README.md`/`.gitignore`/`Dockerfile` com o ícone do
tema, pasta por nome (`docs`, `Properties`, `wwwroot`, `bin`, `.idea`) fechada e aberta, o mesmo SVG
na árvore/aba/Find in Files/Search Everywhere, e o desenho a **16×16**, carregado (`naturalWidth > 0`),
sem corte e centrado no texto da linha (delta < 1,5 px).

`src` e `tests` não têm linha própria na árvore: são ancestrais dos projetos e o `diskRows` os omite
para não duplicar o que a linha do projeto já representa. O E2E cobre o mecanismo de pasta com as
pastas que a árvore desenha; o unit cobre `src`/`tests` pelo nome.

### Build de produção (`dist/`)

O caminho que só o bundle de produção exercita é o da URL do asset: os componentes montam
`/file-icons/<nome>.svg` e `/folder-icons/<nome>.svg`, e o Tauri serve `frontendDist: "../dist"` na
raiz — o mesmo esquema de um servidor estático na raiz do `dist/`.

`bun run build` gerou `dist/file-icons/` (30) e `dist/folder-icons/` (30). Servindo `dist/` na 3020 e
buscando os 60 arquivos: **60 respostas 200, 60 byte a byte iguais** ao arquivo do repositório.

### App real (Linux, Tauri) — não verificado

O binário desta worktree foi compilado (`bunx tauri build --debug --no-bundle`, `target/debug/openridr`)
e aberto com `OPENRIDR_HOME=/tmp/openridr-icons-v2` e uma cópia de `fixtures/OrderDesk`. **A árvore
não abriu:** o app para no *setup gate* ("Setting up OrderDesk"), que exige `csharp-ls` e `netcoredbg`
verificados — esta worktree tem `src-tauri/resources/{csharp-ls,dotnet}` vazios (os `vendor-*.sh` são
download de rede e não foram rodados), e o gate não avança sem um clique, que esta sessão não tem
como injetar (não há `ydotool`/`dotool` no host, só `wtype` para teclado).

Consequência: **não há captura do app Tauri.** O que cobre o risco real (o asset resolver no bundle de
produção) é a verificação do `dist/` acima, que usa exatamente as mesmas URLs e a mesma raiz que o
shell serve. A instância foi encerrada (`kill` no PID próprio; `pgrep -lx openridr` vazio) e o
`OPENRIDR_HOME` temporário removido na próxima execução.

## Capturas

- `depois-*.png` — Playwright em modo web, 1440×900, **DPR 1**.
- `2x/depois-*.png` — a mesma árvore com `OPENRIDR_FILE_ICONS_DPR=2`, **DPR 2** (árvore de 560×1616).
- Sem captura do app Tauri (motivo acima).

**Não verificado:** macOS/WKWebView. O alvo verificado do produto é macOS ARM64 e esta sessão rodou
em Linux arm64; o WKWebView não foi exercitado com os SVGs novos. O caminho de asset é o mesmo
`public/` que o `app-icon.png` já usava, e o `dist/` foi conferido servindo os 60 arquivos.
