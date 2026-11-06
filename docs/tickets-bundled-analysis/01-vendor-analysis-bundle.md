# 01: Runtime 10.0.14 e csharp-ls 0.28.0 embutidos no build

**What to build:** `scripts/vendor-analysis.sh` baixa (ou reusa) os dois artefatos pinados, confere hash, extrai para `src-tauri/resources/{dotnet,csharp-ls}/`, assina todo Mach-O e falha o build em qualquer divergência. `tauri.conf.json` copia as duas árvores e roda o script em `beforeBuildCommand`.

**Blocked by:** None.

**Status:** implementado e verificado em 2026-11-05 — evidência em `docs/evidence/bundled-analysis.md`; notarização/quarentena/Mac limpo real ficaram sem prova (ver "Não verificado").

- [x] `scripts/vendor-analysis.sh` com SHAs fixados (runtime 10.0.14 SHA-512 oficial; nupkg csharp-ls 0.28.0 SHA-512 já usado em `tools.rs`), falha em hash errado, é no-op com marker válido.
- [x] `Resources/dotnet/`: muxer `dotnet`, `host/fxr/10.0.14`, `shared/Microsoft.NETCore.App/10.0.14`, `LICENSE.txt`, `ThirdPartyNotices.txt`.
- [x] `Resources/csharp-ls/`: payload `tools/net10.0/any/*` do nupkg, sem `.nupkg` duplicado, sem shim.
- [x] Assinatura ad-hoc (local) ou Developer ID + hardened + timestamp (`APPLE_SIGNING_IDENTITY`) em todo Mach-O; `codesign --verify --strict` passa.
- [x] `vendor/dotnet-runtime/`, `vendor/csharp-ls/` e as árvores `src-tauri/resources/*` ficam fora do git.
- [x] `bun run tauri build --bundles app` produz um `.app` com `Contents/Resources/dotnet` e `Contents/Resources/csharp-ls`.
- [x] Prova de execução fora do app: `<hostroot>/dotnet <Resources>/csharp-ls/CSharpLanguageServer.dll --version` responde 0.28.0 usando só o runtime embutido.
