# SPEC — Embutir o netcoredbg no app

Status: ready-for-agent
Data: 2026-11-04. Alvo: macOS Apple Silicon (o único alvo distribuído). Windows, Intel e Linux continuam fora.
Decisão do Helder: embutir o `netcoredbg-osx-arm64` 3.2.0-1092; aposentar o download no Debug; manter o csharp-ls como download do usuário.

## Problem Statement

Hoje o primeiro Debug depende de rede e do GitHub: o clique baixa `netcoredbg` 3.2.0-1092 (~3,4 MB), extrai, verifica e só então depura. Sem rede, ou com o GitHub indisponível, o Debug não começa — mesmo com build pronto. O adaptador também vive em `<app home>/tools`, fora do artefato assinado, então a versão que roda não é necessariamente a que o app distribui.

## Solution

O `netcoredbg` 3.2.0-1092 (osx-arm64) passa a ser embutido no bundle, com versão e SHA-256 fixados e verificados no build, assinado com Developer ID + hardened runtime e coberto pela notarização do app. O Debug resolve o adaptador a partir de `Contents/Resources/netcoredbg/netcoredbg`; se o bundle não tiver o binário (ex.: `tauri dev`), usa a instalação anterior em `<app home>/tools` como fallback. Não há download novo no Debug. O csharp-ls continua com o plano revisável e o download do usuário.

## User Stories

1. Como pessoa sem rede, quero iniciar o Debug depois de um build verde, sem baixar nada.
2. Como pessoa que já baixou o netcoredbg numa versão anterior, quero que o app use o binário embutido e não o antigo.
3. Como pessoa rodando `tauri dev`, quero um fallback para o adaptador já instalado quando o bundle de desenvolvimento não tiver resources.
4. Como pessoa usando o app assinado, quero que o adaptador embutido tenha a mesma assinatura/notarização do app e não seja bloqueado pelo Gatekeeper.
5. Como mantenedor, quero que o build falhe se o zip oficial não bater o SHA-256 fixado.
6. Como mantenedor, quero o aviso MIT do netcoredbg e dos componentes embutidos distribuído com o app.
7. Como pessoa que abre Workspace tools, quero ver que o adaptador é embutido, com versão e checksum, e não um plano de download.
8. Como mantenedor, quero o tamanho do DMG antes/depois medido no mesmo método.

## Implementation Decisions

- **Artefato oficial fixado.** `vendor/netcoredbg/netcoredbg-osx-arm64-3.2.0-1092.zip` (3.389.504 bytes, SHA-256 `f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`) é versionado. O build verifica o hash antes de extrair; hash divergente falha o build.
- **Extração no build.** Um script extrai para `src-tauri/resources/netcoredbg/` (ignorado pelo git), assina `netcoredbg` e `libdbgshim.dylib` (Developer ID quando disponível, senão ad-hoc) com hardened runtime, e o Tauri copia para `Contents/Resources/netcoredbg/`. `beforeBuildCommand` roda o script; `tauri dev` não depende dele.
- **Resolução no host.** O host procura primeiro o bundle (`<exe>/../Resources/netcoredbg/netcoredbg`, com override `OPENRIDR_BUNDLED_NETCOREDBG` para teste), depois `<app home>/tools/netcoredbg/<versão>/netcoredbg/netcoredbg`. `--version` continua sendo verificado antes de lançar.
- **Sem download no Debug.** Com o binário embutido presente, o estado do adaptador é `ready`, o plano de instalação não é oferecido e `debug.install`/`debug.retry` recusam com "bundled; nothing to install". O download antigo permanece apenas como código de fallback histórico, nunca acionado pelo Debug.
- **UI.** Workspace tools mostra "Bundled with OpenRidr", versão e SHA-256 do executável embutido; o painel do Debug não mostra progresso/retry de download quando o adaptador é embutido.
- **Assinatura/notarização.** O binário aninhado é reassinado com Developer ID, hardened runtime e timestamp; a notarização do app cobre o conjunto. Entitlements só entram se a execução assinada exigir (JIT/memória executável); o mínimo necessário, nunca um plist copiado.
- **Licença.** `THIRD-PARTY-NOTICES.md` com MIT do netcoredbg e dos componentes embutidos (Roslyn/dbgshim), copiado para `Contents/Resources`.
- **Escopo de arquitetura.** Só osx-arm64: o release 3.2.0-1092 não publica osx-x64. Windows/Linux ficam como estão.
- **csharp-ls intacto.** Plano revisável, download, progresso e retry não mudam.

## Testing Decisions

- **Host:** resolução bundle → fallback → ausente; `ready_adapter` usa o embutido sem plano; `debug.install` recusa quando embutido; `view` reporta bundled/versão/hash; verificação de `--version`.
- **Build:** script falha com zip/hash errado; bundle contém `Contents/Resources/netcoredbg/netcoredbg` e `libdbgshim.dylib`; `codesign --verify --deep --strict` passa; `codesign -dv` do binário aninhado mostra Developer ID/hardened.
- **App real:** home isolada sem `tools/netcoredbg`; Debug inicia e imprime a saída do debuggee; Workspace tools diz bundled; DMG montado (somente leitura/quarentena) também depura.
- **Tamanho:** DMG antes/depois pelo mesmo método (`hdiutil ... UDZO`), app instalado e `Contents/Resources`.
- **e2e:** painel do Debug não oferece download quando bundled; estado `ready` sem rede.

## Out of Scope

- Windows, macOS Intel, Linux; assets x64.
- csharp-ls e o runtime .NET 10 da análise.
- Auto-update independente do netcoredbg.
- Publicar release, push, PR, substituir `/Applications`.
- Embutir o SDK .NET (nunca).

## Further Notes

- Baseline medido em 2026-11-04, mesmo método: app release 14,68 MiB, DMG `5.671.389` bytes. A proposta estimou +3,4 MB no DMG e +9,14 MiB no app.
- O zip oficial não traz `LICENSE`; o texto MIT foi obtido do repositório upstream no mesmo tag.
- Riscos: execução a partir de `Contents/Resources` somente leitura; JIT/entitlements sob hardened runtime; notarização de binário aninhado. Todos precisam de prova no DMG real.
