# 02: Host resolve o par embutido e o setup não baixa

**What to build:** `crates/host/src/bundled.rs` resolve `Resources/csharp-ls` e `Resources/dotnet`; `tools` fica `ready` depois de um `--version` real e recusa plano/instalação/reinstall; `AnalysisRuntime::for_analysis` prefere o runtime embutido e cai para o externo; `lsp.start` executa `<hostroot>/dotnet <entry.dll>` com o SDK do projeto resolvido por `global.json`.

**Blocked by:** 01 (resources no bundle; os testes usam override, então pode começar antes).

**Status:** implementado e verificado em 2026-11-05 — evidência em `docs/evidence/bundled-analysis.md`; notarização/quarentena/Mac limpo real ficaram sem prova (ver "Não verificado").

- [x] Testes red/green: bundle presente → `state=ready` sem rede e sem `tools/` no app home; `plan/install/retry/setup.reinstall` recusam; hash alterado → não `ready`.
- [x] `AnalysisRuntime::for_analysis` prefere o bundle (`bundled=true`, versão 10.0.14, raiz em `Contents/Resources`) e cai para o inventário externo sem bundle.
- [x] `hostroot::ensure` aceita a raiz embutida e continua exigindo exatamente o SDK do projeto + runtime 10 listado.
- [x] `lsp.start` com bundle usa o muxer do hostroot e o DLL do bundle; sem bundle, o caminho instalado da alpha.8 não muda.
- [x] `tauri dev` sem resources mantém o fluxo de download/plano (teste de host com override ausente).
- [x] `cargo test -p openridr-host` e `bun run check` verdes; debug embutido sem regressão.
