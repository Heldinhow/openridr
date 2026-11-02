# SPEC — Solution Explorer fiel ao Rider

Referência: `.scratch/rider-solution-ref/` (4 prints do Rider, não versionados). Alvo: macOS ARM64.

## Auditoria (antes)

| Área | Hoje | Rider |
|---|---|---|
| Linha de projeto | duas camadas, chips `kind`/TFM/restore em destaque | uma linha: ícone `C#` + nome |
| Raiz / pastas de solução | `OrderDesk · N projects`; pasta `src` sem chevron nem contagem | chevron, ícone de pasta de solução, `· N projects` |
| Dependencies | `<details>` com texto corrido | nó com chevron e filhos (Projects / Packages) |
| Arquivos do projeto | lista plana só com o nome | pastas aninhadas, pastas antes dos arquivos |
| Scratches and Consoles | ausente | nó no fim |
| Barra | locate, expand, collapse | show all files, buscar, locate, expand, collapse, ⋮, minimizar |
| Show all files | inexistente | bin/obj/.idea/dotfiles com `no index` e fundo destacado |
| Menu de contexto | inexistente na árvore | menu de projeto com submenu Edit |

## Decisões (grill, respondido com prints + docs)

1. **Itens sem backend ficam fora**, exceto `Run Unit Test` (desabilitado, como no Rider). Fora: Add, Unload, EF Core, Run Multiple, More Run/Debug, Publish, Advanced Build, Local History, Refactor, Inspect, Reformat, Diagrams, Tools, Cut/Copy/Paste/Delete/Rename, Open In, Replace in Files, Edit in Right Split.
2. **Backend existente reaproveitado**: Manage NuGet → painel NuGet; Reload Project → re-avaliação do modelo (`evaluation.run`; o host não avalia projeto isolado); Build/Run/Debug usam o fluxo existente com uma `RunConfig` explícita do projeto clicado (o startup project não muda; o gate de trust continua valendo mesmo com config explícita); Edit `<proj>.csproj` → abre o arquivo; Find in Files → busca de texto; Copy Path; Properties → detalhes do projeto.
3. **Restricted**: Build/Run/Debug aparecem desabilitados com o motivo existente (`buildDisabledReason`/`runAvailabilityReason`/`debugDisabledReason`); em Restricted o menu ganha "Trust this project" (mesmo `grantTrust` dos avisos). Nada executa sem trust.
4. **Build/Run/Debug só em projeto console avaliado** (o host builda pelo TargetPath do startup). Biblioteca/teste: desabilitado com o motivo.
5. **Scratches and Consoles**: não há backend nem pasta de scratches; o nó ficaria falso → fora (já registrado em decisions).
6. **Show all files** exige um método de host novo, somente leitura e sem trust (como `workspace.files`): `workspace.tree {dir}` lista um diretório do workspace (nome, caminho, dir, `hidden`). `hidden` = pasta em `IGNORED_DIRS` ou dotfile/dotfolder; a UI marca `no index` em `hidden` ou em arquivo que não é fonte avaliada (`.sln` e `.csproj` nem são listados). Não lê `.gitignore` (limite declarado).
7. **Chips**: `kind`/TFM viram sufixo cinza `· console · net8.0` na mesma linha (sem quebrar os `toContainText` existentes); avisos de erro/restore continuam visíveis.
8. **Buscar**: filtro client-side por nome na árvore já carregada (não é a busca global).
9. **Minimizar** = esconder a tool window (`toggleToolWindow`). **⋮** = Re-evaluate, Project details, Expand/Collapse All, Show All Files.
10. Zero dependência nova; `data-testid` existentes preservados.

## Aceite

- Dependencies lista projetos e pacotes em linhas planas (sem grupos Projects/Packages).
- Extras além do print: menu de raiz/arquivo (Open, Reload Solution, Copy Path, Find in Files) e navegação por teclado no submenu.
- Linha única por projeto/pasta/arquivo, contagem `· N projects` na raiz e em cada pasta de solução.
- Dependencies expansível com filhos reais; pastas aninhadas nos fontes.
- Barra com os 7 controles; show all files lista bin/obj/dotfiles com `no index` e fundo destacado.
- Botão direito num projeto abre o menu (submenu Edit incluso); Restricted bloqueia Build/Run/Debug com motivo e oferece Trust.
- E2E novo em dados isolados (`tests/e2e/solution-explorer.spec.ts`) + suíte existente verde; `cargo test` do `workspace.tree`.

## Tickets

- 38 — Árvore: linha única, contagens, pastas aninhadas, Dependencies com filhos.
- 39 — Barra: show all files (+`workspace.tree`), buscar, ⋮, minimizar.
- 40 — Menu de contexto de projeto/arquivo com submenu e gates de trust.
- 41 — E2E isolado + docs + review + retro.
