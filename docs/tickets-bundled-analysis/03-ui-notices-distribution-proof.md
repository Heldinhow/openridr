# 03: UI, licenças, docs de distribuição e prova no app/DMG

**What to build:** SetupGate e Workspace tools mostram "Bundled with OpenRidr" (versão, SHA-256, caminho) sem progresso/retry/reinstall; `THIRD-PARTY-NOTICES.md` cobre csharp-ls e o runtime; docs de distribuição deixam de prometer download de análise/runtime externo; evidência com app real, DMG e tamanho medido.

**Blocked by:** 01, 02.

**Status:** implementado e verificado em 2026-11-05 — evidência em `docs/evidence/bundled-analysis.md`; notarização/quarentena/Mac limpo real ficaram sem prova (ver "Não verificado").

- [x] `ToolInstall.bundled` e `AnalysisRuntime.bundled` no `client.ts`; SetupGate/Workspace tools dizem "Bundled and verified", Reinstall desabilitado com motivo; `data-testid` existentes preservados.
- [x] `THIRD-PARTY-NOTICES.md`: csharp-ls 0.28.0 (MIT) e runtime .NET 10.0.14 (MIT) com os textos de licença do tarball.
- [x] `docs/RELEASE-ALPHA.md`/`install-app.md`: nada de "baixa o csharp-ls no primeiro uso" nem "exige runtime .NET 10 externo" para a build com bundle; o que for publicado continua identificado como alpha.8.
- [x] Prova desktop (`verify-openridr`, app Tauri real): `OPENRIDR_HOME` vazio, sem runtime 10 visível (`DOTNET_ROOT`/`PATH` sem 10), setup ready sem download, diagnóstico/semântica na fixture, reabertura sem setup, Debug com breakpoint.
- [x] Prova de identidade: muxer em uso e o DLL vêm de `Contents/Resources` (log do host + `ps`/`lsof`).
- [x] `.app` e DMG medidos (hdiutil UDZO), DMG montado somente leitura executa análise.
- [x] Evidência em `docs/evidence/bundled-analysis.md`, com comandos, exit codes e o que não foi provado (notarização, quarentena, Mac limpo real).
