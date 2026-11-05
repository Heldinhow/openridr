# SPEC — Ícones da Solution Explorer (referência T3 Code)

Status: em implementação. Base: origin/main `60e784f`, branch `feat/solution-file-icons`. Alvo: macOS ARM64.
Continua [SPEC-SOLUTION-EXPLORER-VISUAL](SPEC-SOLUTION-EXPLORER-VISUAL.md) (D6–D9 continuam valendo); esta
spec muda o **desenho dos glifos**, as **linhas de pasta** e a **seleção**.

## Problema

A árvore atual distingue arquivos por glifos Lucide de traço misturados com o C# de traço, e toda pasta
carrega um ícone de pasta. A referência visual enviada pelo Helder (árvore do T3 Code, MIT) é mais limpa:
pasta é só chevron, arquivo é um glifo preenchido de 16 px, e o tipo do `.cs` é o hexágono roxo com o
logotipo C#.

## Decisões

**D1 — Glifos de arquivo vêm do @pierre/trees (Apache-2.0).** Os SVGs usados (`file`, `json`, `markdown`,
`docker`, `yml`, `text`, `git`) são vendorizados em `src/icons/fileGlyphs.tsx`, com o texto Apache-2.0 e o
aviso de origem em `THIRD-PARTY-NOTICES.md`. Nada do Rider/Zed; nenhum pacote novo: `@pierre/trees` **não**
entra como dependência (é beta). Onde o glifo do pacote não existe (`.csproj`, `.sln`, `.razor`, `.resx`,
`.props`, `.sh`) usamos o SVG de documento vendorizado ou o Lucide que já é dependência.

**D2 — Hexágono C# é o padrão do `.cs`.** O glifo `C#` de traço sai; entra o hexágono preenchido do
Simple Icons (`C#`, CC0, via sprite do T3 Code). O tipo de símbolo (`class`/`interface`/`enum`/`record`/
`struct`, PR #23) **não se perde**: vira um selo de 9 px sobreposto no canto inferior direito, com a cor e a
letra do tipo, `title` com o nome do tipo e o `data-symbol-kind` que já existia na linha. Sem tipo
reconhecido, o `.cs` fica só com o hexágono. Isso substitui o "`.cs` com tipo → badge como ícone
principal" do D8 de [SPEC-SOLUTION-EXPLORER-VISUAL](SPEC-SOLUTION-EXPLORER-VISUAL.md).

**D3 — Pasta é chevron e nome, nada mais.** Pastas de fonte, pastas do `.sln` e o modo "Show all files"
perdem o ícone de pasta; o slot de 16 px continua reservado (vazio), então nome de pasta e nome de arquivo
seguem alinhados — é o que a referência mostra. Projetos mantêm o ícone de tipo (D4 de
SPEC-SOLUTION-EXPLORER-VISUAL): na referência essas linhas são pastas, aqui são projetos, e o tipo é
informação de produto. A raiz (solução) mantém o glifo da solução.

**D4 — Guia de indentação.** Linha de 1 px na coluna do chevron de cada ancestral, cobrindo a linha inteira,
desenhada só em linhas com `level ≥ 1`. Pulso puramente CSS (`.tree-guide`), sem medir layout no React.

**D5 — Seleção é contorno.** A linha atual/selecionada ganha `box-shadow: inset 0 0 0 1px var(--accent)`
sobre `--select`, mantendo o raio `--r-md`: é o contorno azul arredondado da referência. `inset` evita
reflow; nenhum testid muda.

**D6 — Paleta: os hex do T3 Code, um token por uso.** Escuro é o tema distribuído e usa os valores exatos
do T3 Code (`csharp #9d6afb`, `json #ffa359`, `markdown #5ecc71`, `docker #69b1ff`, `doc/text #adadb1`,
`git #d5512f`); a captura de referência foi tirada nesse tema. Claro usa os pares do T3 Code e fica como
**não verificado** (Settings não oferece tema claro). Contraste escuro medido sobre `--panel` #0b0b0b:

| Token | Escuro | Contraste | Claro | Contraste |
|---|---|---|---|---|
| `--icon-csharp` | `#9d6afb` | 5.52 | `#693acf` | 6.76 |
| `--icon-json` | `#ffa359` | 9.97 | `#d47628` | 3.27 |
| `--icon-markdown` | `#5ecc71` | 9.70 | `#199f43` | 3.45 |
| `--icon-docker` | `#69b1ff` | 8.75 | `#1a85d4` | 3.93 |
| `--icon-doc` | `#adadb1` | 8.80 | `#84848a` | 3.72 |
| `--icon-git` | `#d5512f` | 4.73 | `#ff8c5b` | 2.29 |

Escuro ≥ 4,5:1 (o `--icon-git` no `--raised` fica em 4,43:1); claro ≥ 3:1, o mínimo de WCAG 1.4.11 para
ícone (não é texto). Os valores claros não foram vistos no app.

**D7 — Mapeamento por nome antes de extensão.** `Dockerfile`, `docker-compose*.yml`, `*.DotSettings*`,
`.gitignore`/`.gitattributes`/`.gitmodules`, `.editorconfig`, `global.json`/`appsettings*.json` são
resolvidos por nome; o resto por extensão, sem ler conteúdo. `fileKind` continua devolvendo o tipo,
`fileGlyph` decide o glifo — os dois puros e testáveis sem DOM.

## Mapa arquivo → glifo

| Arquivo | Kind | Glifo | Origem |
|---|---|---|---|
| `.cs`, `.csx` | csharp | hexágono `C#` | Simple Icons, CC0 |
| `.sln`, `.slnx`, `.csproj`/`.fsproj`/`.vbproj`, `*.DotSettings[.user]`, desconhecido | solution / project / dotsettings / file | documento com dobra | @pierre/trees, Apache-2.0 |
| `.json`, `.jsonc` (`global.json`, `appsettings*.json`) | json | chaves `{ }` | @pierre/trees |
| `.md`, `.markdown` | markdown | `M↓` | @pierre/trees |
| `Dockerfile`, `docker-compose*.yml`/`.yaml`, `*.dockerfile` | docker | baleia | @pierre/trees |
| `.yml`, `.yaml` | yaml | diagrama de blocos | @pierre/trees |
| `.txt` | text | página com linhas | @pierre/trees |
| `.gitignore`, `.gitattributes`, `.gitmodules` | git | ramo git | @pierre/trees |
| `.xml`, `.props`, `.targets` | xml / props | `CodeXml` | Lucide, ISC |
| `.razor`, `.cshtml` | razor | `CodeXml` | Lucide |
| `.resx`, `.resources` | resource | `Package` | Lucide |
| `.editorconfig`, `.config` do usuário | config | `Settings` | Lucide |
| `.sh`, `.ps1`, `.cmd`, `.bat` | script | `SquareTerminal` | Lucide |

## Aceite

1. Toda pasta (fonte, solução, Show all files) renderiza só chevron + nome; nenhum `<svg>` na linha.
2. Nome de pasta e nome de arquivo alinham (slot de ícone vazio preservado) e cada nível ≥ 1 desenha um
   `.tree-guide` por ancestral.
3. `.cs` mostra o hexágono; com símbolo conhecido, o selo de letra sobrepõe no canto e a linha mantém
   `data-symbol-kind` e `title` com o tipo.
4. `.json`, `.md`, `Dockerfile`, `docker-compose*.yml`, `.csproj`, `.DotSettings.user`, `.yml`, `.xml`,
   `.sh`, `.gitignore` e desconhecido recebem os glifos da tabela; teste unitário cobre nome e extensão.
5. Linha atual/selecionada com contorno `--accent` arredondado, sem mudar altura.
6. Nenhuma dependência nova; `bun run check`, `bun run test:unit` e o E2E novo nas portas 1490/4387 passam.
7. Origem e licença de cada glifo registradas em `decisions.md` e `THIRD-PARTY-NOTICES.md`.
8. Captura antes/depois em `docs/evidence/solution-file-icons/`; app real (agent-desktop) quando o
   orquestrador liberar — até lá, "não verificado".

## Referências consultadas (somente leitura)

- T3 Code `apps/web/src/pierre-icons.ts` e `components/chat/PierreEntryIcon.tsx` (MIT): lógica de
  mapeamento por nome/extensão e paleta claro/escuro por tipo.
- `@pierre/trees` 1.0.0-beta.6, `dist/builtInIcons.js` (Apache-2.0): os SVGs vendorizados.
- Simple Icons (CC0): glifo C# tal como distribuído no sprite do T3 Code.
- Lucide (`lucide-react`, ISC): glifos de xml/razor/recurso/config/script que já estavam no app.
