# 12: SDK do projeto x runtime das ferramentas

**What to build:** Um projeto preso ao SDK 8.0.408 no `global.json` abre, avalia, builda, roda e analisa sem o app exigir SDK 10. A análise (csharp-ls) exige só o **runtime** .NET 10, que é da ferramenta, não do projeto.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] O host identifica o runtime `Microsoft.NETCore.App` 10 via `--list-runtimes` para a análise e para o root composto.
- [ ] O gate da análise e o plano do csharp-ls não exigem mais um SDK 10 instalado.
- [ ] O SDK do projeto continua vindo do `global.json`/TargetFramework, com a resolução atual (incluindo `rollForward`).
- [ ] Descoberta cobre `DOTNET_ROOT`, `/usr/local/share/dotnet`, `/opt/homebrew` (e `libexec`), `~/.dotnet`, SDKs gerenciados e todas as entradas do PATH, deduplicadas por raiz canônica.
- [ ] O root composto usa muxer/fxr/runtime 10 da raiz do runtime e `sdk/`/`packs` da raiz do projeto.
- [ ] Projecto 8.0.408 com runtime 10 presente analisa; projeto 8 sem runtime 10 pede só o runtime.
- [ ] Testes de unidade para descobridores/resolvedores e um teste de host com raízes falsas.
- [ ] Nenhuma mudança no gate do SDK do projeto para build/run/test.
