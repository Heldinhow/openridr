# 12: SDK do projeto x runtime das ferramentas

**What to build:** Um projeto preso ao SDK 8.0.408 no `global.json` abre, avalia, builda, roda e analisa sem o app exigir SDK 10. A análise (csharp-ls) exige só o **runtime** .NET 10, que é da ferramenta, não do projeto.

**Blocked by:** None (can start immediately).

**Status:** done

- [x] O host identifica o runtime `Microsoft.NETCore.App` 10 via `--list-runtimes` para a análise e para o root composto.
- [x] O gate da análise e o plano do csharp-ls não exigem mais um SDK 10 instalado.
- [x] O SDK do projeto continua vindo do `global.json`/TargetFramework, com a resolução atual (incluindo `rollForward`).
- [x] Descoberta cobre `DOTNET_ROOT`, `/usr/local/share/dotnet`, `/opt/homebrew` (e `libexec`), `~/.dotnet`, SDKs gerenciados e todas as entradas do PATH, deduplicadas por raiz canônica.
- [x] O root composto usa muxer/fxr/runtime 10 da raiz do runtime e `sdk/`/`packs` da raiz do projeto.
- [x] Projecto 8.0.408 com runtime 10 presente analisa; projeto 8 sem runtime 10 pede só o runtime.
- [x] Testes de unidade para descobridores/resolvedores e um teste de host com raízes falsas.
- [x] Nenhuma mudança no gate do SDK do projeto para build/run/test.

## Notes

- `Inventory` lê `--list-runtimes` por raiz; `AnalysisRuntime` substitui `HostSdk` e só considera `Microsoft.NETCore.App` 10.x estável. `sdk.host.version` passa a ser a versão do runtime e `sdkPath` fica nulo (card/status bar ficam para o ticket 13).
- Plano do csharp-ls: `dotnet tool install` roda com qualquer SDK detectado; a verificação `--version` usa `DOTNET_ROOT` da raiz do runtime 10. Sem SDK e sem runtime são erros distintos.
- Provas: `cargo test -p openridr-host --lib` 213/213; integração `sdk_inventory_...`, `hostroot_composes_runtime10_...`, `tools_plan_cancel...`, `tools_install_is_real_verified_and_reused`, `project_sdk_missing_keeps...`, `lsp_start_requires_tool_sdk_and_current_restore`, `lsp_loads_the_real_solution...`; E2E `workspace.spec.ts` 16/16 e `trust-sdk.spec.ts` 3/3; `bun run check`.
- Não verificado: app Tauri real e uma instalação com raiz só-runtime (essa combinação é provada com raízes falsas nos testes de descoberta/`describe`; a análise real usa a raiz de runtime detectada). Windows/macOS Intel continuam fora.
