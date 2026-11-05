# SPEC — Solution Explorer visual (Rider/VS Code)

Status: em implementação. Base: `feat/rider-parity` b65ab07. Alvo: macOS ARM64.
Diagnóstico que originou a spec: [GAPS-SOLUTION-EXPLORER-VISUAL.md](GAPS-SOLUTION-EXPLORER-VISUAL.md).
Referência de comportamento: Rider (observação conhecida, sem captura versionada nesta branch) e VS Code
open source (Seti e codicons, MIT). Nenhum asset é copiado.

## Problema

A Solution Explorer mostra chrome de diagnóstico (`MSBuild evaluated`, `Evaluate`, `revision N`,
`Project details`) e usa um glifo só para tudo (projeto, classe, solução). Rider mostra uma árvore silenciosa
em que o tipo é comunicado por ícone e cor.

## Decisões

**D1 — A árvore não mostra estado de avaliação.** Saem do sidebar: a tag `MSBuild evaluated` /
`Text outline · not evaluated`, a linha `.eval-controls` (Evaluate/Re-evaluate, `Evaluated · revision N`,
`Project details`) e os dois rodapés (`MSBuild has not run…`, `Partial workspace…`). O estado já existe,
duplicado, na barra de status; a fonte única visível passa a ser ela.

**D2 — Testids preservados, ações realocadas.** `eval-run` vira um botão-ícone de refresh na barra de
status (antes da leitura de avaliação, sempre visível, desabilitado com o mesmo motivo de hoje). `eval-state`
fica dentro do segmento `status-evaluation` com o texto completo (inclusive `revision N`, que os specs leem
por regex) — o wrapper mantém `data-testid="status-evaluation"` e o `title` que o E2E de layout verifica.
`Project details` sai do cabeçalho e fica só no menu ⋮ (item já existente, `tree-more-details`); o painel
`eval-details` continua. Assim nenhum testid é renomeado/removido e os 4 specs que clicam a ação passam a
abrir o ⋮ antes (mudança de spec explícita).

**D3 — Estados bloqueantes continuam na árvore.** `eval-disabled-reason` (SDK ausente/solução vazia) e o
aviso `eval-stale` são erros acionáveis, não chrome; ficam, mas sem crescer a linha normal. O erro de
avaliação por projeto (`eval-project-error-*`) e o aviso de restore continuam nas linhas/nós.

**D4 — Projeto por tipo, com `web` vindo do texto.** O ícone do projeto distingue `console` (app),
`library`, `test`, `web`, `unknown`. `web` não existe no modelo avaliado; vem do outline textual
(`<Project Sdk="Microsoft.NET.Sdk.Web">`), que já lê o `.csproj` (`crates/host/src/outline.rs`). `web` vence
`console`, porque projeto web é `OutputType=Exe`; `test` vence os dois. Nenhuma propriedade MSBuild nova é
consultada. Saem das linhas os chips `· console` e `· net8.0`; o tipo/TFMs viram atributos
(`data-project-kind`, `data-project-tfms`) para testes e leitores de tela (`title` não muda de contrato).

**D5 — Dependências ganham tipos, não grupos.** Filhos planos (decisão anterior do Solution Explorer)
continuam: referência de projeto usa o ícone do projeto referenciado (quando conhecido), pacote usa ícone
de pacote e versão discreta, e cada TFM avaliada vira uma linha de framework derivada
(`Microsoft.NETCore.App (net8.0)`, `Microsoft.AspNetCore.App (net8.0)` para `web`, `.NET Standard (…)`,
`.NET Framework (…)`). O texto `· project` / `· package` fica (contrato dos specs). Agrupar
Frameworks/Packages/Projects exigiria nova decisão e reescrever specs; fica fora.

**D6 — Paleta semântica, não cores soltas.** Tokens `--icon-*` em `:root`, com `light-dark(claro, escuro)`
e fallback escuro declarado antes (WebKit antigo cai no fallback). Valores são derivados das cores do Seti
(MIT) e ajustados para contraste ≥ 6:1 sobre `--panel` (#0b0b0b) no escuro e ≥ 6:1 sobre `#fff` no claro;
o build só distribui o tema escuro (Settings: "Light theme is unavailable"), então o claro é
forward-compatible, **não verificado**. Nenhuma cor de arquivo é usada para texto de linha; nomes ficam
`--fg`/`--muted`.

**D7 — Ícone por símbolo C# quando há evidência barata.** Método novo read-only do host
`workspace.symbolKinds { paths: string[] }` → `{ kinds: { "<path relativo>": "class" | "interface" | "enum" |
"record" | "struct" }, scanned: N }`. Lê só `.cs` dentro do workspace, no máximo 256 arquivos e 256 KB cada
(acima disso, sem tipo). Parser conservador em `crates/host/src/outline.rs`: ignora comentários de linha,
procura declaração de tipo no texto e prefere a cujo nome é o stem do arquivo. Sem declaração confiável, não
devolve nada e a UI mantém o glifo C#. Não usa LSP (que exige trust/documento aberto) e não compila.
Custo: um read por fonte na primeira carga da revisão; a UI busca uma vez por conjunto de fontes e cacheia.

**D8 — Badges de letra são desenho próprio.** Classe/interface/enum/record/struct = quadrado arredondado
16×16 com a letra (`C`, `I`, `E`, `R`, `S`) na cor do tipo; não é SVG, fonte nem código do Rider/VS Code.
Glifos de arquivo continuam Lucide (`lucide-react`, ISC) e `CSharpIcon` (MDI, Apache-2.0); solução/folders/
Dependencies/pacotes continuam Lucide, agora com tint. Zero dependência nova.

**D9 — Gerado/excluído esmaece.** `no index` (Show All em `bin`/`obj`/dotfiles, ou arquivo fora das fontes
avaliadas) mantém o fundo âmbar atual e passa a reduzir nome, ícone e badge para `--muted`/opacidade 0.6.
Cores de tipo não somem; só perdem intensidade.

**D10 — Acessibilidade.** Cada ícone de tipo tem `aria-hidden` (o nome já está na linha); o botão de
reavaliar tem `aria-label`; o botão de detalhes no ⋮ mantém o nome. Contraste de ícones ≥ 4,5:1 (tabela
abaixo, medido). `role="tab"` do cabeçalho inferior não muda.

## Paleta (escuro → claro)

Medido com WCAG 2.1 (sRGB) sobre `--panel` escuro `#0b0b0b` e sobre `#ffffff`.

| Token | Escuro | Contraste | Claro | Contraste | Uso |
|---|---|---|---|---|---|
| `--icon-solution` | `#7aa7ff` | 8.25 | `#1f4f9e` | 7.86 | raiz `.sln`/`.slnx` |
| `--icon-solution-folder` | `#9db4dd` | 9.38 | `#3d5680` | 7.39 | solution folder |
| `--icon-folder` | `#9a9a9a` | 6.99 | `#5f6368` | 6.05 | pasta de fontes |
| `--icon-project-console` | `#7cc0ff` | 10.16 | `#12558c` | 7.77 | projeto app/console |
| `--icon-project-library` | `#8ed39a` | 11.19 | `#20663a` | 6.95 | biblioteca |
| `--icon-project-test` | `#e0b76e` | 10.47 | `#7a4f00` | 7.13 | teste |
| `--icon-project-web` | `#7fd3e8` | 11.61 | `#0f5e70` | 7.35 | web |
| `--icon-project-unknown` | `#9a9a9a` | 6.99 | `#5f6368` | 6.05 | desconhecido |
| `--icon-dependencies` | `#c0a8f0` | 9.47 | `#5b3a9e` | 8.26 | nó Dependencies |
| `--icon-package` | `#e0b24a` | 9.97 | `#7a5200` | 6.92 | pacote NuGet |
| `--icon-framework` | `#9db4dd` | 9.38 | `#3d5680` | 7.39 | framework/TFM |
| `--icon-symbol-class` | `#6f9dff` | 7.44 | `#1f4f9e` | 7.86 | badge `C` |
| `--icon-symbol-interface` | `#5fc9a8` | 9.75 | `#186a52` | 6.52 | badge `I` |
| `--icon-symbol-enum` | `#e5b64b` | 10.43 | `#7a5200` | 6.92 | badge `E` |
| `--icon-symbol-record` | `#c58af2` | 7.77 | `#63338f` | 8.75 | badge `R` |
| `--icon-symbol-struct` | `#ee9292` | 8.59 | `#8f2f2f` | 8.03 | badge `S` |
| `--icon-csharp` | `#6fb3d9` | 8.55 | `#17607e` | 6.97 | `.cs` sem tipo |
| `--icon-json` | `#d6c14a` | 10.86 | `#6f5c00` | 6.55 | `.json` |
| `--icon-xml` | `#e08a58` | 7.45 | `#8a3f0f` | 7.50 | `.xml`, `.config`, `.props`, `.targets` |
| `--icon-razor` | `#b48cf0` | 7.45 | `#5b3a9e` | 8.26 | `.razor`, `.cshtml` |
| `--icon-resource` | `#73c7a3` | 9.77 | `#186a52` | 6.52 | `.resx`, `.resources` |
| `--icon-markdown` | `#6fb3d9` | 8.55 | `#17607e` | 6.97 | `.md` |
| `--icon-yaml` | `#b48cf0` | 7.45 | `#5b3a9e` | 8.26 | `.yml`, `.yaml` |
| `--icon-config` | `#8fa1a8` | 7.34 | `#465a61` | 7.25 | `.editorconfig`, `.gitignore` |
| `--icon-script` | `#8dc149` | 9.24 | `#3f6b12` | 6.32 | `.sh`, `.ps1` |
| `--icon-text` | `#a3a3a3` | 7.80 | `#5f6368` | 6.05 | `.txt` e genérico |

## Mapa ícone → tipo

| Nó / arquivo | Glifo | Token |
|---|---|---|
| solução (raiz, `.sln`, `.slnx`) | `Blocks` (Lucide) | solution |
| solution folder | `FolderTree` | solution-folder |
| pasta de fontes | `Folder` | folder |
| projeto console/app | `SquareTerminal` | project-console |
| projeto library | `BookMarked` (novo no `Icon.tsx`) | project-library |
| projeto test | `FlaskConical` | project-test |
| projeto web | `Globe` (novo) | project-web |
| projeto desconhecido | `Box` | project-unknown |
| Dependencies | `Network` | dependencies |
| referência de projeto | ícone do projeto referenciado (`ProjectIcon`) | kind do referenciado |
| pacote | `Package` | package |
| framework | `Layers` (novo) | framework |
| `.cs` com tipo | badge de letra | symbol-* |
| `.cs` sem tipo | `CSharpIcon` | csharp |
| `.json` | `Braces` | json |
| `.xml`/`.config`/`.props`/`.targets` | `CodeXml` | xml |
| `.razor`/`.cshtml` | `CodeXml` | razor |
| `.resx` | `Package` | resource |
| `.md` | `FileText` | markdown |
| `.yml`/`.yaml` | `SlidersHorizontal` | yaml |
| `.editorconfig`/`.gitignore`/`.gitattributes` | `Settings` | config |
| `.sh`/`.ps1`/`.cmd` | `SquareTerminal` | script |
| `.txt`/sem extensão | `File` | text |

## Contrato do host (novo)

`workspace.symbolKinds` (read-only, sem trust, como `workspace.files`/`workspace.tree`):

```json
{ "paths": ["src/App/Program.cs", "src/App/Foo.cs"] }
→ { "kinds": { "src/App/Program.cs": "class" }, "scanned": 2 }
```

- Só `.cs`; caminho resolvido e confinado ao workspace (`resolve_in_workspace`); fora → erro por caminho
  inválido? Não: a lista ignora o item fora do workspace (uma árvore não deve falhar inteira por um path).
  Para o teste de contenção, a resposta nunca inclui chave fora do workspace.
- Limite: 256 caminhos, 256 KB por arquivo; excedeu, ignora (não mente "unknown").
- Tipos: `class`, `interface`, `enum`, `record`, `struct`. `record class`/`record struct` → `record`.
- Parser prefere a declaração cujo nome bate com o stem; senão, a primeira declaração de tipo do arquivo.
- Sem tipo confiável → chave ausente.

## Aceite

1. Árvore não contém `MSBuild evaluated`, `Evaluate`, `Re-evaluate`, `revision N` nem `Project details`.
   Status bar mantém `eval-run` e `eval-state` (com `revision N`) e o botão de reavaliar funciona em
   Restricted (desabilitado com motivo) e trusted.
2. Projetos: ícone e cor por tipo; `console`/`net8.0` não aparecem como texto na linha; atributos
   `data-project-kind`/`data-project-tfms` presentes; erro/restore continuam visíveis.
3. Dependencies: referência com ícone do projeto (quando conhecido), pacote com ícone/cor, framework por TFM;
   `· project`/`· package` preservados.
4. `.cs` com declaração reconhecida mostra badge C/I/E/R/S; sem reconhecimento, C#.
5. `.sln`/`.csproj`/`.json`/`.xml`/`.razor`/`.cshtml`/`.resx`/`.props`/`.targets`/`.md`/`.yml`/`.sh`
   diferenciados por ícone e cor; desconhecido cai no genérico.
6. `no index` esmaece ícone e nome além do fundo.
7. Nenhuma dependência nova; `bun run check` passa; testes Rust do parser/host passam; teste unitário do
   mapeamento passa; specs E2E atualizados (execução pendente de liberação da porta).
8. Verificação visual no app real (agent-desktop) quando o orquestrador liberar.

## Referências consultadas (somente leitura, nada copiado)

- VS Code Seti: `extensions/theme-seti/icons/vs-seti-icon-theme.json` (MIT) — mapeamento
  `.cs`→`_c-sharp`, `json`→`_json`, `xml`→`_xml`, `razor`→`_html`, com os hex usados como ponto de partida.
- vscode-codicons `src/template/mapping.json` (MIT) — nomes `symbol-class`, `symbol-interface`,
  `symbol-enum`, `symbol-struct`, `symbol-namespace`; só a semântica, nenhum SVG/fonte.
- Seti UI (jesseweed, MIT) — `mapping.less`/`vs-seti-icon-theme.json` para extensões e cores.
- O Rider não é open source; nenhuma captura, string de UI ou asset do Rider é usado. A comparação é de
  comportamento observado.
