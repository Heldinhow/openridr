# SPEC — Projetos fora do diretório da solution

Issue-mãe: [#52](https://github.com/Heldinhow/openridr/issues/52) — "Evaluate projects outside the solution directory".

## Problem Statement

Uma solution grande costuma referenciar projetos que não ficam no mesmo diretório dela (`..\Other\Other.csproj` no `.sln`). Hoje o OpenRidr recusa esses projetos na avaliação: o projeto aparece como falho ("resolves outside the workspace"), sem arquivos na árvore, e a solution inteira fica travada — `require_restored` nunca passa, então build, run, debug, testes e análise não rodam nem para os projetos que estão dentro do diretório da solution.

O host já executa MSBuild nesses projetos: `restore.start` sem `project` roda `dotnet restore <sln>`, que avalia e restaura a solution inteira (medido numa fixture: o restore cria `obj/project.assets.json` no projeto de fora). O que falta é o host **modelar** o resultado e deixar as superfícies de leitura, escrita e execução usarem esses arquivos, sem afrouxar o confinamento.

## Solution

O workspace passa a ter um **escopo**: a raiz (diretório da solution) mais o diretório canônico de cada projeto nomeado pelo `.sln`/`.slnx`. A raiz continua sendo o diretório da solution para a árvore, a busca, o cwd do build e do terminal; o escopo é o conjunto de diretórios que o host pode tocar.

Com a solution confiada (*Trusted*), o escopo inteiro é autorizado pela mesma *Trust decision*: não nasce prompt novo, `trusted.json` não muda de formato, e o texto do trust passa a dizer que ele cobre a solution e os diretórios dos projetos que ela referencia. Em *Restricted mode* nada muda.

Projetos dentro do escopo são avaliados, aparecem na árvore com seus arquivos, abrem no editor, recebem análise e breakpoints, e build/run/debug/testes funcionam. Caminho de arquivo fora da raiz é sempre absoluto na API e na UI; nenhum `..` atravessa a fronteira.

## User Stories

1. Como dev com uma solution grande, quero que um projeto referenciado fora do diretório da solution seja avaliado, para ver TFMs, dependências, TargetPath e estado de restore dele.
2. Como dev, quero ver esse projeto na Solution Explorer com os arquivos dele, para navegar sem sair da IDE.
3. Como dev, quero abrir e salvar um arquivo desse projeto no editor, para trabalhar nele como em qualquer outro.
4. Como dev, quero ver diagnósticos do csharp-ls nesse arquivo, para corrigir erro sem trocar de ferramenta.
5. Como dev, quero ir para a definição e achar referências de um símbolo que vive nesse projeto, para navegar código entre projetos.
6. Como dev, quero renomear um símbolo que aparece nesse projeto e revisar a prévia, para refatorar com segurança.
7. Como dev, quero pôr breakpoint num arquivo desse projeto e depurar, para investigar o comportamento dele.
8. Como dev, quero que erros e avisos de build desse projeto apareçam no painel Problems e naveguem para o arquivo, para não caçar no log.
9. Como dev, quero rodar (Run) o projeto console que está fora da solution, para executar o programa.
10. Como dev, quero buildar e depurar esse projeto, para o fluxo de execução funcionar igual ao de um projeto interno.
11. Como dev, quero que o restore da solution cubra esse projeto, para os gates de restore não travarem o resto da solution.
12. Como dev, quero que o painel NuGet liste e consiga planejar/aplicar pacotes nesse projeto, para não receber "fora do workspace" num projeto que é da solution.
13. Como dev, quero que editar um `.csproj` ou um `.cs` desse projeto marque o modelo como stale, para reavaliar quando o projeto muda.
14. Como dev, quero ver na árvore que o projeto está fora do diretório da solution (caminho absoluto visível), para entender por que ele não está sob a pasta da solution.
15. Como dev, quero que a aba do arquivo mostre o nome do arquivo e o caminho absoluto no tooltip, para saber de qual projeto ele veio.
16. Como dev que confia numa solution, quero que a decisão de trust cubra os projetos que ela referencia, para não ser perguntado de novo a cada diretório.
17. Como dev, quero que o texto do trust diga exatamente o que ele autoriza, para decidir com informação.
18. Como dev, quero que em Restricted mode nada execute nem seja analisado, mesmo com projetos fora do diretório, para abrir código desconhecido com segurança.
19. Como responsável pela segurança, quero que nenhum caminho fora do escopo seja lido, escrito, analisado ou depurado, para o confinamento não regredir.
20. Como responsável pela segurança, quero que `..`, caminho absoluto fora do escopo e symlink que leva para fora continuem recusados com a mesma resposta de hoje, para não abrir oráculo de existência.
21. Como responsável pela segurança, quero que um projeto cujo diretório é a raiz do filesystem seja recusado, para uma solution não transformar `/` em escopo.
22. Como responsável pela segurança, quero que o escopo seja derivado só do `.sln` confiado e limitado a 64 diretórios extras, para uma solution não varrer o disco.
23. Como mantenedor, quero o confinamento implementado uma vez e usado por todas as fronteiras, para não existirem duas regras divergindo.
24. Como mantenedor, quero testes de host que provem cada recusa com fixture real, para o PR de risco alto ter evidência verificável.
25. Como mantenedor, quero que a árvore, a busca e o git continuem ancorados na raiz da solution, para o escopo não virar uma segunda raiz de navegação.

## Implementation Decisions

- **Escopo do workspace.** A estrutura do workspace aberto ganha um escopo ordenado de diretórios canônicos: a raiz primeiro, depois o diretório pai de cada projeto nomeado pelo `.sln`/`.slnx`, na ordem do arquivo, sem duplicar e sem repetir um diretório já contido num anterior. Projeto cujo diretório canônico é a raiz do filesystem é recusado (não avaliado, com motivo). O escopo é limitado a 64 diretórios extras; o excedente fica não avaliado com motivo textual, nunca truncado em silêncio. O escopo é calculado ao abrir o workspace e recalculado a cada `evaluation.run` (o `.sln` é a fonte de verdade; projeto adicionado depois é pego sem reabrir).
- **Trust cobre o escopo (decisão do Helder, 2026-11-06).** A *Trust decision* gravada para o caminho da solution autoriza o escopo inteiro. Não há segundo prompt, não há decisão por diretório e `trusted.json` mantém o formato mapa `caminho → trusted|restricted`. Justificativa registrada: o host já executa MSBuild nesses projetos via `dotnet restore <sln>`; os diretórios vêm do `.sln` confiado, não de input do usuário; e "uma pergunta por projeto" continua valendo. O texto do trust (painel, popover e prompt) passa a declarar que a autorização cobre a solution e os diretórios dos projetos que ela referencia.
- **Restricted mode inalterado.** Sem avaliação, sem análise, sem build/run/debug/testes. Leitura de arquivo como texto continua permitida dentro do escopo, como hoje é dentro da raiz; nada de fora do escopo é tocado.
- **Caminho absoluto fora da raiz.** Caminho de projeto/arquivo/diagnóstico fora da raiz é devolvido absoluto (canônico) na API e usado como chave na UI; dentro da raiz continua relativo com `/`. Nenhuma API aceita `..` para atravessar a fronteira: caminho relativo é sempre interpretado contra a raiz e recusado por forma quando normaliza para fora dela; arquivo de projeto de fora é endereçado por caminho absoluto.
- **Confinamento com escopo, uma implementação.** A função de confinamento passa a receber o escopo em vez da raiz e mantém as três regras atuais: (1) caminho relativo que normaliza para fora da raiz é recusado por forma, sem I/O; (2) o ancestral existente mais próximo precisa canonicalizar dentro de **algum** diretório do escopo, então symlink que leva para fora é recusado do mesmo jeito exista ou não o alvo (sem oráculo de existência); (3) o alvo é canonicalizado e checado de novo. Todas as fronteiras de caminho passam a usar essa função: leitura/escrita/criação/renomeação/exclusão/reveal, listagem de diretório, symlinks, símbolos da árvore, análise (didOpen/didChange/completion/definition/references/rename/code actions/workspace symbol), breakpoints e NuGet.
- **Avaliação.** O planejador aceita projeto cujo caminho canônico está no escopo (o `.sln` o nomeia) e mantém `failed` só para arquivo ausente/ilegível e para os limites acima. A lista de fontes do projeto avaliado passa a incluir arquivos do escopo (antes filtrava `starts_with(root)`), então o projeto de fora aparece com seus `.cs`, e `relative_path` devolve absoluto para o que está fora da raiz.
- **Outline e árvore.** O outline de texto lê o `.csproj` de fora quando ele está no escopo e lista os `.cs` do diretório dele; a árvore mostra o projeto com o nome vindo do `.sln` e o diretório absoluto como subtítulo/tooltip, sem mudar `data-testid` existentes.
- **Análise.** O csharp-ls já recebe a solution inteira (`--solution`), então os diagnósticos de arquivos de fora já chegam; o que muda é o host aceitá-los e devolver navegação para eles (definição, referências, rename, code actions) quando o caminho está no escopo. Diagnóstico de caminho fora do escopo continua descartado.
- **Breakpoints e DAP.** Breakpoint, condição e hit count passam a aceitar arquivo no escopo; o caminho absoluto já é a chave usada pelo adaptador para fonte fora da raiz, e o mapeamento de frame canônico já devolve absoluto nesse caso. Run to cursor e a verificação de breakpoint seguem a mesma chave.
- **Problemas de build.** Caminho de erro/aviso e link de log passam a ser aceitos quando estão no escopo; fora dele continuam texto não navegável, como hoje.
- **NuGet.** O resolvedor de projeto do NuGet usa o escopo, então planejar e aplicar pacote num projeto de fora funciona; o painel deixa de listar um projeto que não consegue planejar.
- **Watch e stale.** O host passa a observar o `.csproj` de fora e os diretórios de fonte no escopo (mesmo fingerprint por mtime/tamanho e por listagem de diretório), então editar arquivo do projeto de fora marca o modelo stale como dentro da raiz. O cap de diretórios observados continua valendo.
- **Restore, build, run, debug e testes.** O gate de restore passa a ver os assets do projeto de fora (antes o projeto sem caminho absoluto virava `unknown` e travava tudo). Build usa o `.csproj` absoluto com cwd na raiz; run usa TargetPath e cwd no diretório do projeto; debug usa o assembly avaliado; testes usam os mesmos gates. Nenhum comando novo, nenhum restore implícito.
- **Snapshot.** O snapshot do workspace expõe o escopo como lista de caminhos absolutos canônicos (raiz primeiro), para a UI marcar projeto de fora e para os testes provarem o recálculo. Nenhum `data-testid` novo é obrigatório; o selo "fora da solution" usa o caminho do projeto comparado à raiz.
- **Sem dependência nova, sem Playwright.** Tudo em Rust/TS existentes; a prova de UI é o app Tauri real dirigido por `agent-desktop`.

## Testing Decisions

- **Bom teste aqui é comportamento observável de fronteira**: o projeto de fora aparece avaliado com fontes e TargetPath; um arquivo do escopo abre e salva; um caminho fora do escopo é recusado com o mesmo código de hoje; um symlink que leva para fora é recusado exista ou não o alvo; o gate de restore deixa de travar a solution.
- **Fixture nova `fixtures/OutsideSolution`**: `app/App.sln` (raiz do workspace) com `app/App.csproj` (console, `net8.0`, referencia `../lib/Lib.csproj`) e `lib/Lib.csproj` (biblioteca), cada um com uma fonte; `app/global.json` fixa o SDK `8.0.408` como as demais fixtures. A raiz é `app/`, então `lib/` é o projeto de fora — o mesmo formato do bug.
- **Host (`cargo test -p openridr-host`)**: unidade para o cálculo do escopo (ordem, dedupe por contenção, cap 64, recusa da raiz do filesystem, recálculo quando o `.sln` muda); boundary em `crates/host/tests/host.rs` com a fixture real para avaliação, outline, árvore, leitura/escrita, NuGet, problemas de build e breakpoints; matriz de confinamento (dentro do escopo funciona; fora do escopo recusa; `..` recusa; absoluto fora recusa; symlink de dentro para fora recusa; symlink da raiz para diretório do escopo funciona; alvo existente e inexistente fora respondem igual).
- **Trust**: o teste de propriedade existente (`cargo test -p openridr-host --test trust_model`) ganha o caso "trust na solution autoriza o escopo e nada fora dele"; Restricted continua sem executar nada.
- **UI (`bun test tests/unit`, `bun run check`)**: helpers de caminho (absoluto fora da raiz não é confundido com relativo; aba e árvore montam o nome certo), sem teste de componente novo.
- **App real (`agent-desktop`, superfície C do `verify-openridr`)**: abrir a fixture, confiar, ver o projeto de fora na árvore, abrir/editar/salvar um arquivo dele, ver diagnóstico, pôr breakpoint, buildar e rodar; capturas em `docs/evidence/projects-outside-solution/`. Uma instância por vez, state próprio da issue.
- **Não perseguir falhas conhecidas** de `docs/evidence/test-stability.md`; registrar e seguir. Playwright não é prova desta feature.

## Out of Scope

- Busca Files/Text, git status/gutter/diff e "Show all files" fora da raiz: continuam ancorados na raiz da solution, declarados como limite.
- `ProjectReference` para projeto fora que **não** está no `.sln`: o MSBuild continua construindo o referenciado, mas ele não é modelado nem aparece na árvore nesta rodada.
- cwd do terminal continua na raiz da solution.
- Redesenho do fluxo de trust: só o texto muda.
- Windows e macOS Intel: não verificados.
- Playwright e novos specs E2E.

## Further Notes

- **Risco alto (trust).** O PR toca o modelo de confiança: precisa de aprovação do Helder e a Evidência tem que mostrar o confinamento (o que abre, o que recusa, com fixture real), não só o caminho feliz.
- **Limite declarado sobre symlink de projeto.** O escopo vem do caminho canônico do `.csproj`; um `.sln` que nomeia um projeto através de symlink coloca em escopo o diretório real do alvo. É a mesma decisão que o `.sln` já toma para o restore; fica registrado, não é acidente.
- **Projeto em diretório ancestral da raiz.** Se o `.sln` nomeia um projeto num diretório acima da solution, esse diretório entra no escopo inteiro (mesma granularidade dos demais). Consequência declarada, não caso especial.
- **Evidência** em `docs/evidence/projects-outside-solution.md`, retro em `docs/RETRO-PROJECTS-OUTSIDE-SOLUTION.md`, notas de implementação em `docs/implementation-notes.md`.
