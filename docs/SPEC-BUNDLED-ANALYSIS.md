# SPEC — Análise C# embutida (csharp-ls + runtime privado)

Status: ready-for-agent
Data: 2026-11-05. Alvo: macOS Apple Silicon (o único alvo distribuído). Windows, Intel e Linux continuam fora.
Decisão do Helder: embutir o `csharp-ls` 0.28.0 **e** o runtime .NET 10 que ele exige no app/DMG; o primeiro uso não baixa analisador, runtime nem adaptador. O SDK do projeto continua externo e resolvido por `global.json`.

## Problem Statement

Na alpha.8 o Debug já não baixa nada (`netcoredbg` embutido), mas a análise C# ainda depende de rede no primeiro uso: o setup baixa o pacote `csharp-ls` 0.28.0 (~21,6 MB, ~99 MB extraídos) do NuGet e exige um runtime .NET 10 **externo** (`Microsoft.NETCore.App` 10.x) para executá-lo. Em outra máquina o link do log abre o download no navegador e o "Retry setup" falha sem mensagem útil. Sem rede, sem NuGet disponível ou sem runtime 10 instalado, não há análise — nem diagnóstico, nem semântica.

## Solution

O `csharp-ls` 0.28.0 e um runtime `Microsoft.NETCore.App` 10.0.14 privado (osx-arm64) passam a ser embutidos em `Contents/Resources/`, com versão e checksum fixados e verificados no build. O host resolve o analisador e o runtime do bundle primeiro; o `hostroot` composto continua sendo o que junta o **SDK do projeto** (externo, `global.json`) ao runtime 10 (agora do bundle). O setup verifica o par uma vez e fica `ready` sem download, sem plano e sem popup; `tools.plan`/`install`/`retry`/`setup.reinstall` recusam com mensagem clara quando o analisador é embutido. A UI mostra "Bundled with OpenRidr" com versão, SHA-256 e caminho.

## User Stories

1. Como pessoa sem rede, quero abrir um projeto e ter análise/diagnóstico no primeiro uso, sem baixar nada.
2. Como pessoa sem runtime .NET 10 instalado, quero análise mesmo assim; o SDK do projeto continua sendo o que o `global.json` pede.
3. Como pessoa que já instalou o csharp-ls numa alpha anterior, quero que o bundle vença o cache antigo e não seja executado o binário baixado.
4. Como pessoa rodando `tauri dev`, quero continuar usando o plano/instalação por download quando o bundle de desenvolvimento não tem os resources.
5. Como mantenedor, quero que o build falhe se o tarball do runtime ou o nupkg do csharp-ls não baterem o checksum fixado.
6. Como mantenedor, quero licenças e avisos redistribuíveis (MIT dos dois) publicados com o app.
7. Como mantenedor, quero medir o DMG antes/depois pelo mesmo método e provar que o binário em uso é o do bundle.

## Implementation Decisions

- **Runtime privado pinado.** `dotnet-runtime-10.0.14-osx-arm64.tar.gz` oficial (33.251.887 bytes, SHA-512 `19c5781d…52b0382`, conferido com `releases-index.json` → `Runtime/10.0.14/*.sha512`) extraído para `src-tauri/resources/dotnet/` (muxer, `host/fxr/10.0.14`, `shared/Microsoft.NETCore.App/10.0.14`, `LICENSE.txt`, `ThirdPartyNotices.txt`). Versão 10.0.14 = latest LTS do canal 10.0 no metadata oficial na data.
- **Analisador pinado.** nupkg `csharp-ls` 0.28.0 (21.643.814 bytes, SHA-512 `a138b9c8…170a99`) já fixado em `tools.rs`; o build extrai `tools/net10.0/any/*` (payload gerenciado, 57 MB) para `src-tauri/resources/csharp-ls/`. Sem shim do NuGet: o host executa `<hostroot>/dotnet <Resources>/csharp-ls/CSharpLanguageServer.dll`.
- **Download no build, não no uso.** Os dois artefatos são baixados pelo script de vendor (hash conferido) para `vendor/` (não versionados: 55 MB) e as árvores extraídas ficam em `src-tauri/resources/` (ignoradas pelo git). O app/DMG não contém downloader de análise.
- **Resolução no host.** `crates/host/src/bundled.rs`: resources = `<exe>/../Resources` (macOS arm64), com overrides de debug `OPENRIDR_BUNDLED_CSHARP_LS` e `OPENRIDR_BUNDLED_DOTNET_ROOT`. Analisador embutido = `csharp-ls/CSharpLanguageServer.dll`; runtime embutido = raiz com muxer + `host/fxr/10.x` + `shared/Microsoft.NETCore.App/10.x` (versão lida do diretório).
- **Runtime para análise.** `AnalysisRuntime::for_analysis` prefere o bundle e cai para o inventário externo (o `tauri dev` sem resources continua funcionando). O `hostroot` composto não muda: SDK do projeto (externo) + `host/fxr`/`shared` do runtime escolhido, em links no app home; o muxer é copiado.
- **Falha fechada no app distribuído.** `bundled::availability()` distingue `Bundled`, `Absent` (binário de desenvolvimento) e `Damaged` (app sem uma metade). `Damaged` recusa setup, plano, instalação, retry, reinstall e início de análise com orientação de reinstalar o app; nunca cai no download nem usa runtime 10 externo. `Absent` mantém o fallback de `tauri dev`.
- **Integridade do par.** O estado `ready` acompanha entrada, dependências gerenciadas, muxer e runtime por fingerprint (caminho/tamanho/mtime) a cada snapshot; o SHA-256 de conteúdo do par é calculado uma vez na verificação e registrado no log. Reescrever a árvore preservando mtimes está fora do escopo (o manifest do build cobre a distribuição).
- **Sem download no setup.** Com o par embutido, `tools.view().state = ready` (após `--version` real, uma vez por sessão), `plan = null`, sem progresso; `tools.plan`, `tools.install`, `tools.retry` e `setup.reinstall` recusam com "bundled; nothing to install". Sem bundle (dev), o fluxo de download da alpha.8 fica intacto.
- **UI.** SetupGate e Workspace tools dizem "Bundled with OpenRidr", versão e SHA-256 do DLL embutido, caminho em `Contents/Resources`; Reinstall fica desabilitado com motivo.
- **Assinatura.** Todo Mach-O embutido (muxer, `libhostfxr`, `libhostpolicy`, dylibs nativas e o payload do csharp-ls se tiver nativo) é assinado no script de vendor com Developer ID + hardened runtime + timestamp quando há identidade, senão ad-hoc; o Tauri copia depois. Entitlements só se a execução provar necessidade (JIT do runtime é o risco novo).
- **Licença.** `THIRD-PARTY-NOTICES.md` ganha csharp-ls 0.28.0 (MIT) e o runtime .NET 10.0.14 (MIT) com os textos de `LICENSE.txt`/`ThirdPartyNotices.txt` do próprio tarball, que também viajam em `Contents/Resources/dotnet/`.
- **Escopo de arquitetura.** Só osx-arm64. Sem SDK embutido, sem MSBuild próprio, sem trocar csharp-ls por outra versão.

## Testing Decisions

- **Host (red/green):** com override para um resources temporário, `tools.view()` = ready sem rede/instalação, `still_ready` acompanha o hash, `plan/install/retry/reinstall` recusam; `AnalysisRuntime::for_analysis` prefere o bundle e cai para o externo na ausência; `hostroot` composto a partir da raiz embutida ainda lista SDK 8 e runtime 10 e rejeita SDK extra.
- **Build:** script falha com hash errado; resources contêm muxer, fxr, shared, DLL do csharp-ls; `codesign --verify --strict` nos binários embutidos.
- **App real:** `OPENRIDR_HOME` isolado e vazio, `PATH`/`DOTNET_ROOT` sem runtime 10 (simulação por env, não Mac limpo): trust → setup ready sem download → diagnóstico/semântica na fixture → reabertura sem setup. Prova de identidade: a linha de log/tools mostra o caminho em `Contents/Resources`, e `lsof`/`ps` do muxer aponta o bundle.
- **Debug embutido:** continua funcionando (regressão), com breakpoint e saída do debuggee.
- **Tamanho:** DMG e `.app` antes/depois com `hdiutil ... UDZO`.

## Out of Scope

- Windows, macOS Intel, Linux.
- Embutir o SDK .NET ou MSBuild; `global.json` continua do projeto.
- Notarização/publicação de release (exige credencial/Apple; só se as ferramentas locais permitirem).
- Remover o código de download/plano do csharp-ls (continua como fallback de `tauri dev`).
- Auto-update das ferramentas embutidas (passam a acompanhar a release do app).

## Further Notes

- Baseline alpha.8: DMG 10.699.101 bytes (10,20 MiB), `.app` 24,25 MiB. Estimativa: +137 MB extraídos (runtime 83 MiB + csharp-ls 57 MiB) → DMG na casa de 55–65 MiB; medir, não estimar.
- Risco principal: runtime .NET 10 sob hardened runtime (JIT) e execução a partir de `Contents/Resources` somente leitura (DMG montado). Precisa de prova no volume final.
- Risco secundário: `hostroot` com links para dentro do bundle; se o app mudar de caminho, o marker invalida e recompõe (inputs incluem `runtimeRoot`).
- O zip do csharp-ls no NuGet não traz LICENSE; o texto MIT vem da expressão de licença do nuspec (`MIT`, `razzmatazz/csharp-language-server`).
