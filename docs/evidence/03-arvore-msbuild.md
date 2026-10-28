# Evidência — Ticket 03: árvore MSBuild real

Implementação local em 28/10/2026. Fonte normativa: [ticket 03](../tickets/03-arvore-msbuild.md), [SPEC](../SPEC.md) P0.1/P0.2, “Avaliação e ferramentas”, “Estado, erros e processos” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Node / Bun | v26.10.0 / 1.3.9 |
| Rust | cargo 1.99.0, rustc 1.99.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 (canal `chrome`) / 1.64.1 |
| SDK do host | 10.0.100 (e 8.0.416) em `/usr/local/share/dotnet` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (fixado por `global.json`, `rollForward: disable`) |
| Fixtures | `fixtures/OrderDesk` (sln + console + lib + testes), `fixtures/MissingSdk`, nova `fixtures/BrokenImport` (fora do `OrderDesk.sln`) |
| Base de código | `93c493a` (ticket 02) + esta fatia |

Log: [`logs/03-environment.log`](logs/03-environment.log). Nenhum SDK/ferramenta foi instalado nesta tarefa.

## O que foi construído

- **Avaliação MSBuild por projeto** (`crates/host/src/evaluation.rs`): a solução é lida como texto para projetos, pastas de solução, `NestedProjects` e configurações (`Debug` padrão). `.sln` não aceita `-getProperty`/`-getItem` (MSB1063), então cada projeto roda **um** processo `dotnet msbuild <csproj> -nologo -getProperty:… -getItem:… -p:Configuration=…` com o muxer da raiz do SDK do projeto, cwd no diretório do projeto, `DOTNET_ROOT` dessa raiz, `DOTNET_MULTILEVEL_LOOKUP=0`, `DOTNET_CLI_UI_LANGUAGE=en`. Nada de restore, build, targets ou LSP.
- **Gate e pré-requisito**: `evaluation.run` passa pelo `require_trust`; sem trust → `trust_required` e nenhum processo (inclusive HTTP direto). Sem SDK do projeto resolvido → `sdk_missing`/`invalid_state`, MSBuild não roda e o host SDK 10 nunca é substituto; `global.json` não é reescrito.
- **Modelo e estados**: `evaluation.state` `idle|running|ok|partial|failed`, contexto `{configuration, targetFramework}`, `revision`, `workspaceId`, `contextId`, log global, e por projeto `state ok|failed`, `kind` (`console|library|test|unknown`), TFMs, `targetPath` exatamente avaliado, `evaluatedTfm` (TFM que produziu esse TargetPath), `outputType`, `projectRefs`, `packageRefs` (versão `null` quando desconhecida/central), `sources` (Compile, sem bin/obj), `error`, `log` bruto, `exit`, e `restored` tri-estado (`restored|not_restored|unknown`) com motivo.
- **Multi-TFM**: quando o projeto declara mais de um TFM e nenhum foi selecionado, a avaliação externa devolve `TargetFramework`/`TargetPath` vazios (exit 0). O host **não escolhe**: mostra os TFMs, `TargetPath` ausente e “Select a TFM”; com `targetFramework` selecionado, reavalia com `-p:TargetFramework` e usa o TargetPath/Compile dessa avaliação. TFM selecionado não declarado pelo projeto também não é inventado.
- **Invalidação**: salvar pelo app `.csproj/.props/.targets/.sln/global.json` marca o modelo como `stale` e move o contexto; o polling compara mtime/tamanho de `global.json`, `Directory.Build.props/targets`, `Directory.Packages.props` e arquivos de projeto. Revogar/trocar/fechar zera a avaliação; resultado tardio é rejeitado por token (`stale_context`). Configuração/TFM e Re-evaluate geram nova avaliação; TargetPath antigo aparece apenas rotulado como avaliação anterior/stale.
- **UI**: árvore avaliada solução → pastas → projetos (kind/TFMs, refs, pacotes, fontes), substituindo o contorno textual só após trust+avaliação; modo restrito continua “Text outline · not evaluated”. Controles de configuração/TFM, Re-evaluate, estado textual, banner `Evaluation stale`, banner “Restore not requested — package dependencies and analysis are incomplete”, projeto com falha com causa/log/Open project file/Retry, e painel de detalhes em coluna própria (propriedades, TargetPath, refs, contagem/desconhecidos de pacotes, restore, contexto/revisão).

## Comandos e saídas reais

### JSON do MSBuild (bruto, trecho)

`logs/03-msbuild-json.log` guarda o JSON completo; trecho real (sem edição de valores):

```json
{
  "Properties": {
    "Configuration": "Debug",
    "TargetFramework": "net8.0",
    "TargetFrameworks": "",
    "TargetPath": "/Users/helder/Projetos/openridr/fixtures/OrderDesk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
    "OutputType": "Exe",
    "MSBuildProjectName": "OrderDesk.Cli",
    "IsTestProject": ""
  },
  "Items": {
    "ProjectReference": [
      {
        "Identity": "../OrderDesk.Core/OrderDesk.Core.csproj",
        "FullPath": "/Users/helder/Projetos/openridr/fixtures/OrderDesk/src/OrderDesk.Core/OrderDesk.Core.csproj",
        "Filename": "OrderDesk.Core",
        "Extension": ".csproj"
      }
    ],
    "PackageReference": [],
    "Compile": [
      { "Identity": "Program.cs",
        "FullPath": "/Users/helder/Projetos/openridr/fixtures/OrderDesk/src/OrderDesk.Cli/Program.cs" }
    ]
  }
}
```

O host devolveu o **mesmo** TargetPath para OrderDesk.Cli (valores conferidos no log HTTP):

```text
msbuild Properties.TargetPath = /Users/helder/Projetos/openridr/fixtures/OrderDesk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
host evaluation.projects[OrderDesk.Cli].targetPath = /Users/helder/Projetos/openridr/fixtures/OrderDesk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
```

O mesmo comando foi rodado com `-p:Configuration=Release`: TargetPath avaliado `/…/bin/Release/net8.0/OrderDesk.Cli.dll`, exit 0, sem restore (`logs/03-host-http.log`).

### Multi-TFM sem seleção (exit 0, sem TargetPath)

```json
{
  "Properties": {
    "TargetFramework": "",
    "TargetFrameworks": "net8.0;netstandard2.0",
    "TargetPath": ""
  },
  "Items": { "Compile": [] }
}
```

Com `-p:TargetFramework=netstandard2.0`: `TargetFramework: "netstandard2.0"`, `TargetPath: "…/bin/Debug/netstandard2.0/Multi.dll"` e `Compile` com `C.cs`. O teste Rust `multi_tfm_needs_an_explicit_selection_for_target_path_and_sources` cobre os dois casos e o TFM não declarado.

### Falha de avaliação (MSB4019)

```text
$ dotnet msbuild BrokenImport.csproj -nologo -getProperty:TargetPath -getItem:Compile
exit=1
--- stdout
--- stderr
/Users/helder/Projetos/openridr/fixtures/BrokenImport/BrokenImport.csproj(10,3): error MSB4019: The imported project
"/Users/helder/Projetos/openridr/fixtures/BrokenImport/build/Missing.props" was not found. Confirm that the expression
in the Import declaration "build/Missing.props" is correct, and that the file exists on disk.
```

Log: [`logs/03-msbuild-failure.log`](logs/03-msbuild-failure.log). Nenhum `bin/`/`obj/` foi criado na fixture. O projeto continua na árvore com `state=failed`, `error` com MSB4019, log completo, `Open project file` e `Retry evaluation`; os demais projetos da solução continuam avaliados (`partial`), coberto por teste Rust e pelo E2E.

### Host pela ponte HTTP de desenvolvimento

Log completo: [`logs/03-host-http.log`](logs/03-host-http.log). Trechos reais:

```text
$ POST /call {"method":"evaluation.run","params":{}}   (workspace aberto, sem trust)
{"error":{"code":"trust_required","message":"evaluation.run requires workspace trust; no process was started."},"ok":false}

$ POST /call {"method":"evaluation.run","params":{"targetFramework":"net8.0;Configuration=Release"}}
{"error":{"code":"invalid_state","message":"Configuration and TFM selections must not contain `;`, `=`, quotes or line breaks; MSBuild was not run."},"ok":false}

$ POST /call {"method":"trust.grant","params":{}}      [trecho]
"projectSdk": {"state":"resolved","resolvedVersion":"8.0.408","root":"/Users/helder/Library/Application Support/OpenRidr/sdks/8.0.408"}

$ POST /call {"method":"evaluation.run","params":{}}
state: ok | revision: 1 | workspaceId: 1 | contextId: 2
context: {"configuration":"Debug","targetFramework":null} | stale: false
[OrderDesk.Cli]  ok · exit 0 · 1 project reference · 0 packages · Program.cs
[OrderDesk.Core] ok · exit 0 · 0 project references · 0 packages · Order.cs
[OrderDesk.Tests] ok · exit 0 · 1 project reference · 3 packages · OrderTests.cs (xunit 2.9.2, Test.Sdk 17.11.1, runner 2.8.2)
folders: src (Cli, Core), tests (Tests); kinds console/library/test; TFMs net8.0; restored "restored"

$ POST /call {"method":"evaluation.run","params":{"configuration":"Release"}}
state ok · revision 2 · cliTargetPath /…/src/OrderDesk.Cli/bin/Release/net8.0/OrderDesk.Cli.dll

$ POST /call {"method":"workspace.close","params":{}}
evaluation idle, projects 0
```

### Testes Rust

```text
$ cargo test -p openridr-host
running 15 tests   (unit: parse do JSON MSBuild, multi-TFM, pacotes sem versão, configurações/pastas da sln, MSB4019)
test result: ok. 15 passed; 0 failed; 0 ignored
running 24 tests   (boundary: gate, SDK, avaliação real, espaço no path, sem restore, multi-TFM, falha parcial, token tardio, resultado tardio, stale)
test result: ok. 24 passed; 0 failed; 0 ignored
```

Log: [`logs/03-cargo-test.log`](logs/03-cargo-test.log). Testes reais desta máquina (`#[cfg(target_os = "macos")]`) usam o SDK 8.0.408:

- `evaluation_never_runs_before_trust_or_without_a_resolved_project_sdk` — `trust_required`, depois `invalid_state` sem SDK, estado `failed`, nenhum `bin/obj`.
- `evaluation_with_a_missing_project_sdk_reports_sdk_missing` — `fixtures/MissingSdk`, código `sdk_missing`, `global.json` inalterado.
- `evaluation_returns_the_real_msbuild_model_for_every_project` — 3 projetos, pastas, kinds, TFMs, refs, 3 pacotes com versões, fontes, `restored`, `revision`, Debug→Release.
- `evaluation_of_a_fresh_copy_with_spaces_creates_no_restore_artifacts` — cópia em `Order Desk ; & copy`, TargetPath com espaço, `not_restored`, nenhum `obj/project.assets.json`.
- `multi_tfm_needs_an_explicit_selection_for_target_path_and_sources` — sem seleção: TFMs declarados, `TargetPath`/fontes ausentes e “Select a TFM”; com `netstandard2.0`: `evaluatedTfm` e TargetPath dessa avaliação.
- `a_late_evaluation_does_not_replace_a_newer_selection` — duas avaliações sobrepostas no mesmo contexto: a mais antiga (Debug) é descartada com `stale_context` e a revisão 2 (Release) permanece.
- `a_failed_evaluation_stays_in_the_model_while_other_projects_evaluate` — `partial`, MSB4019/log/exit 1.
- `revoking_trust_while_msbuild_runs_drops_the_late_result` — 8 projetos, revoke durante `running`, resultado tardio `stale_context`, estado final `idle`/0 projetos.
- `saving_a_project_file_or_an_external_edit_makes_the_evaluation_stale` — save pelo host move contexto e marca stale; edição externa detectada por mtime; Release reavalia; editar `.cs` não invalida.

### Playwright (Chrome real + Vite + host + MSBuild real)

```text
$ bun run test:e2e
  ✓ trust evaluates every project and the evaluated TargetPath follows the selection
  ✓ a broken import stays in the tree with cause, log, open and retry
  ✓ saving a project file in a temp copy marks the model stale and re-evaluates it
  ✓ a direct evaluation call is denied before trust
  ✓ … 10 testes dos tickets 01–02 continuam verdes
  14 passed (18.9s)
```

Log: [`logs/03-playwright.log`](logs/03-playwright.log). O fluxo principal usa a fixture real; os casos de save/falha usam cópias em diretório temporário (nunca mutam `fixtures/`). A troca de configuração no E2E é feita por teclado (foco no select + type-ahead “r” para Release). Captura: [`screenshots/03-evaluated-tree.png`](screenshots/03-evaluated-tree.png) — árvore avaliada com pastas, Cli/Core/Tests (fontes, refs, pacotes), state “Evaluated · revision 2”, painel de detalhes do Cli com TFMs, Evaluated TFM, TargetPath Release avaliado, restore e revisão/contexto.

### Frontend

```text
$ bun run build
✓ 52 modules transformed.
✓ built in 742ms
```

Log: [`logs/03-build.log`](logs/03-build.log). Aviso de chunk >500 kB é do CodeMirror.

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| MSBuild só após trust, sem restore implícito; modelo identificado por workspace/contexto/revisão; LSP não é fonte | Gate único antes de qualquer processo; 3 processos `msbuild` por avaliação; `revision/workspaceId/contextId`; cópia fresca sem `project.assets.json` | HTTP, `cargo-test`, E2E 1/4 |
| Solução/pastas/console/lib/testes, fontes, TFMs, refs entre projetos, pacotes; bin/obj ocultos; desconhecido não inventado | Pastas `src`/`tests`; kinds/TFMs/refs/pacotes reais; Compile filtrado; kind/TFM/versão “unknown” quando ausentes | HTTP, E2E 1, screenshot |
| Configuração/TFM explícitos; TargetPath da avaliação, não de convenção; sem snapshot antigo | Configurações da sln (`Debug` padrão); multi-TFM exige TFM; Debug→Release reavalia e troca o TargetPath; stale rotula anterior | HTTP, `cargo-test`, E2E 1/3 |
| Projeto com erro permanece; causa/log/abrir/retry; restore incompleto explícito; sem “solução limpa” | `partial`, erro MSB4019 + log + open + retry; banner “Restore not requested — package dependencies and analysis are incomplete” | `logs/03-msbuild-failure.log`, `cargo-test`, E2E 2/3 |
| Edição/import relevante invalida; recarga explícita; TargetPath invalidado não é atual; resposta tardia não substitui | Save pelo app move contexto + stale; mtime externo detectado; token tardio rejeitado; Re-evaluate explícito | `cargo-test`, E2E 3 |
| G1 — trust/fronteira | Negativo antes de trust (HTTP/UI/teste); revoke durante avaliação descarta resultado; nenhum restore/build; `global.json` intocado | HTTP, `cargo-test`, E2E 4 |
| G2 — contexto/integridade | Token por workspace/contexto; `running` observável; Resultado tardio não aplicado; exit por projeto (`unknown` possível); argumentos separados e cwd explícito | `cargo-test`, HTTP |
| G3 — consentimento/instaladores | Nenhum download/instalação/elevação nesta fatia | ambiente |
| G4 — UI/privacidade | UI inglesa; configuração/TFM/estado por texto (não só cor); controles nativos com foco visível e motivo textual; painel de detalhes com `aria-label`; sem telemetria | E2E, screenshot |
| G5 — validação/proveniência | OS/arch/versões, SDK host/projeto, TFM, comandos, logs, exit, revisão e limitações abaixo | esta página + logs |

ACs tocados: **AC-08** (árvore avaliada e ambiguidade já no 01), **AC-09** (falha/parcialidade; restore executável fica no 04), **AC-10 parcial** (modelo/TargetPath/invalidação; reload pós-restore em 04 e consumidores 11/12/14), **AC-01/07** (negativos deste consumidor), **AC-32/33** (superfície e evidência desta fatia).

## Limitações e pendências honestas

- **Windows não foi executado**. O caminho `dotnet.exe`, `TerminateProcess` e paths CRLF/Windows não têm evidência aqui; o ticket 18 deve repetir avaliação real em Windows x64.
- **macOS Intel não foi exercitado**; a matriz P0 continua pendente (ticket 18).
- **`.slnx` não é suportado**: a abertura aceita `.sln` clássico e `.csproj`; solution filters (`.slnf`) e soluções com mapeamentos de configuração/plataforma não triviais não foram testados. O host passa apenas `Configuration` (sem `Platform`), usando o default do projeto; mapeamentos por projeto da solução não são aplicados.
- **Restore real é ticket 04**. Aqui `restored` vem de `obj/project.assets.json` vs. mtime do `.csproj`; um assets existente não prova que todos os pacotes estão corretos. O banner é deliberado mesmo quando o arquivo existe e está mais novo.
- **Referências resolvidas não são exibidas**: `ReferencePath`/`ResolvedProjectReferencePaths` são saída de target; a árvore mostra apenas `ProjectReference`/`PackageReference` declarados, como o ticket permite (“quando conhecidos”).
- **Avaliação executa imports/property functions do repositório** (Directory.Build.*, SDK targets). É código do repositório e continua atrás do gate de trust, como o SPEC exige.
- **Playwright usa Chrome real contra Vite + host-http**, não a janela Tauri nativa (limitação de captura TCC já registrada nos tickets 01–02). O mesmo `dispatch` é chamado.
- **Acessibilidade**: controles nativos, `aria-label`, foco visível e estado textual; não houve teste com leitor de tela nem auditoria WCAG completa.
- **Retry avalia a solução/projeto inteiro**, não apenas o projeto com erro; a UI rotula “Retry evaluation” e não promete escopo por projeto.
- Fixture `BrokenImport` tem `global.json` 8.0.408 próprio só para permitir a demo de falha; não entra no `OrderDesk.sln` e não é restaurada/compilada pelo app.
