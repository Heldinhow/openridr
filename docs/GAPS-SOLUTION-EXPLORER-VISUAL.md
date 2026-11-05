# Gaps — Solution Explorer visual (Rider/VS Code) — 2026-11-04

Branch `feat/solution-explorer-visual` (base `feat/rider-parity` b65ab07). Escopo do pedido do Helder:
tirar o ruído de avaliação/MSBuild da árvore e diferenciar projetos, pastas, dependências e arquivos por
cor/ícone, referência Rider + VS Code (MIT). Nada aqui foi executado no app ainda (outra sessão usa a
porta 1420); as referências de linha valem no commit base.

## 1. Ruído visível na Solution Explorer

| O que aparece | Onde é gerado | Quando aparece | Rider tem? |
|---|---|---|---|
| `MSBuild evaluated` / `Text outline · not evaluated` | `src/WorkspaceView.tsx:4455-4458` (tag do cabeçalho) | sempre | não |
| Botão `Evaluate project model` / `Re-evaluate` | `src/WorkspaceView.tsx:4495-4512` (`.eval-controls`) | sempre | não (Rider sincroniza sozinho) |
| `Evaluated · revision N` (o "Evaluate Revision") | `src/WorkspaceView.tsx:4513-4520` + `evaluationStateLabel` em `343-356` | sempre em modo avaliado | não |
| Botão `Project details` | `src/WorkspaceView.tsx:4521` | projeto selecionado | não |
| Rodapé `MSBuild has not run and nothing is evaluated.` | `src/WorkspaceView.tsx:4607-4612` | sem árvore avaliada | não |
| Rodapé `Partial workspace · N of M projects…` | `src/WorkspaceView.tsx:4602-4606` | avaliação parcial | não |
| Aviso grande de SDK/avaliação indisponível | `src/WorkspaceView.tsx:4524-4568` | SDK ausente / solução vazia | não (Rider mostra no build) |
| Aviso `Evaluation stale` | `src/WorkspaceView.tsx:4569-4585` | revisão velha | não |
| Painel `Evaluated project details` | `src/WorkspaceView.tsx:5095-5200` | botão acima | não |

Os dois últimos são estados de erro acionáveis; o restante é chrome de diagnóstico. A decisão de manter os
controles visíveis é `docs/decisions.md:314` ("Controles de avaliação continuam visíveis e compactos
(testes clicam `eval-run`/`Project details`)") — o pedido novo substitui isso, e o registro fica em
`decisions.md` com a data desta branch.

Testids afetados (contrato): `eval-run`, `eval-state`, `eval-stale`, `eval-disabled-reason`. `eval-run` é
clicado em 4 specs; `eval-state` é lido (inclusive `revision N` por regex) em ~20 specs; "Project details"
é clicado por role+name em 4 specs. A UI nova precisa manter os três acessíveis (ver SPEC).

## 2. Cores e ícones

| Área | Hoje | Arquivo | Rider | VS Code (Seti/codicons) |
|---|---|---|---|---|
| Raiz da solução | Lucide `Blocks`, cinza | `src/Icon.tsx:65,120`; `src/styles.css:3260` | ícone de solução colorido | Seti não tem `.sln`; codicon `symbol-file` |
| Pasta de solução | Lucide `FolderTree`, cinza | idem | pasta com contagem | sem equivalente |
| Projeto | **sempre** o glifo C# (`CSharpIcon`), sem tipo | `src/FileIcon.tsx:16-19`; `src/SolutionTree.tsx:196` | ícone por tipo (app/lib/test/web) | sem ícone de `.csproj` no Seti; cor vem do tema |
| Tipo do projeto | só texto cinza `· console · net8.0` | `src/SolutionTree.tsx:211-213` | só ícone | — |
| Pasta de fontes | mesmo `Folder` cinza | `src/SolutionTree.tsx:143,167` | pasta cinza | `_folder` cinza |
| Dependencies | `Network` cinza | `src/SolutionTree.tsx:230` | ícone de dependências | — |
| Referência de projeto | `Box` cinza + `· project` | `src/SolutionTree.tsx:238` | ícone do projeto referenciado | — |
| Pacote | `Package`, mesma cor; nome+versão juntos | `src/SolutionTree.tsx:241` | pacote com versão discreta | — |
| Frameworks | **não existe** | — | nó `Frameworks` | — |
| `.cs` (classe/interface/enum/record/struct) | **sempre** o mesmo glifo C# | `src/FileIcon.tsx:18` | letra do símbolo com cor | codicons `symbol-class`, `symbol-interface`, `symbol-enum`, `symbol-struct`, `symbol-record` (MIT) |
| `.sln` | `Blocks` cinza | `src/FileIcon.tsx:6-7` | ícone de solução | Seti não mapeia |
| `.csproj` | `Box` cinza | `src/FileIcon.tsx:5` | ícone de projeto | Seti não mapeia |
| `.json` / `.xml` | `Braces` / `File` — mesma cor | `src/FileIcon.tsx:9,18` | por extensão | Seti: json `#cbcb41`, xml `#e37933`, razor `#519aba` |
| `.razor` / `.cshtml` | `File` genérico | `src/FileIcon.tsx:18` | página Razor | Seti `_html` / `_html_3` |
| `.resx`, `.props`, `.targets`, `.yml`, `.sh` | `File` genérico | idem | por tipo | Seti tem cor/glyph para todos |
| Gerado/excluído | só fundo âmbar (`no index`) | `src/styles.css:3262-3264` | texto esmaecido | VS Code esmaece |

Dados disponíveis hoje:

- `EvaluationProject.kind` = `console | library | test | unknown` (`src/host/client.ts:82-96`,
  `crates/host/src/evaluation.rs:819-828`). Não existe `web`.
- O outline textual classifica por `<Project Sdk>`/`OutputType` (`crates/host/src/outline.rs:92-110`), mas a
  árvore avaliada descarta esse `kind` (`src/SolutionTree.tsx:32-52`). É a via barata para `web` sem reavaliar.
- Não há tipo de símbolo por arquivo no snapshot. `lsp.documentSymbol` existe
  (`src/host/client.ts:918-933`), mas exige trust, LSP ativo e uma chamada por arquivo — caro para a árvore.
- `crates/host/src/outline.rs` já é o lugar de parsing textual conservador; um scan de declaração pode
  alimentar o ícone sem compilar nem abrir documento.

## 3. Alvo (o que a spec fecha)

1. Árvore sem chrome de avaliação: estado vai para a barra de status (já existe lá, duplicado) e ações para
   o menu ⋮. `Project details` continua acessível (⋮) e os testids `eval-run`/`eval-state` continuam na UI
   (barra de status), com `eval-state` preservando `revision N` para os specs.
2. Paleta semântica por tipo (tokens `--icon-*`, `light-dark()` com fallback escuro) e ícones:
   solução, pasta de solução, pasta, projeto por tipo (console/app, library, test, web), Dependencies,
   referência de projeto, pacote, framework, arquivo por extensão e `.cs` por símbolo.
3. `web` sai do texto do `.csproj` (outline) quando a avaliação diz `console`.
4. Tipo de símbolo por `.cs` via método read-only novo do host (`workspace.symbolKinds`), parser
   conservador testado em Rust; sem tipo confiável, mantém o glifo C#.
5. Gerado/excluído (`no index`) esmaece ícone e nome, além do fundo atual.
6. Nenhuma dependência nova de UI; glifos continuam Lucide + `CSharpIcon` (Apache-2.0); badges de letra são
   desenho próprio. Nenhum SVG/fonte do Rider, Seti ou codicons é copiado.

## 4. Fora deste escopo (registrar, não fingir)

- Nó `Frameworks` agrupado e grupos `Projects`/`Packages` dentro de Dependencies: a lista plana é decisão
  anterior (`docs/SPEC-SOLUTION-EXPLORER.md`, decisão 10) e os specs dependem do texto atual. Nesta branch
  só entram linhas de framework e ícones; agrupar exigiria nova decisão + reescrita de specs.
- `web` a partir da avaliação MSBuild (hoje só pelo texto do `.csproj`).
- Ícone por símbolo para arquivos `.cs` gigantes (>256 KB), fora do workspace ou com sintaxe que o scan
  conservador não entende: cai no glifo C#.
- Tema claro: o build só distribui o token set escuro (`SettingsDialog.tsx:316-317`); os valores claros de
  `light-dark()` existem para quando houver um token set claro, mas não foram verificados no app.
