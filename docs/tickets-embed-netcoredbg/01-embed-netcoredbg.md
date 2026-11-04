# 01: Embutir o netcoredbg no bundle

**What to build:** O `netcoredbg` 3.2.0-1092 (osx-arm64) é distribuído dentro do app, assinado e notarizado com ele. O Debug usa o binário do bundle, sem download; se o bundle não tiver resources, cai para a instalação anterior. Workspace tools mostra que é embutido, com versão e checksum. O csharp-ls continua sendo baixado pelo usuário.

**Blocked by:** None (can start immediately).

**Status:** implementado e verificado em 2026-11-04 — evidência em `docs/evidence/embed-netcoredbg.md`.

- [x] `vendor/netcoredbg/netcoredbg-osx-arm64-3.2.0-1092.zip` versionado com o SHA-256 oficial; o build falha se o hash divergir.
- [x] O build extrai e assina `netcoredbg` e `libdbgshim.dylib` (Developer ID + hardened runtime; ad-hoc no build local) antes de copiar para `Contents/Resources/netcoredbg/`.
- [x] O host resolve o adaptador do bundle primeiro e do `<app home>/tools` depois; `--version` é verificado antes de lançar.
- [x] Com o adaptador embutido, o estado é `ready`, não há plano/download no Debug e `debug.install`/`debug.retry` recusam com mensagem clara.
- [x] Workspace tools mostra "Bundled with OpenRidr", versão 3.2.0 e SHA-256 do executável; sem progresso/retry de download.
- [x] `THIRD-PARTY-NOTICES.md` (MIT netcoredbg + Roslyn/dbgshim) vai para `Contents/Resources`.
- [x] `codesign --verify --deep --strict` passa no app; o binário aninhado mostra Developer ID/hardened runtime e a notarização é aceita.
- [x] Debug funciona no app real com `OPENRIDR_HOME` sem `tools/netcoredbg`, imprime a saída do debuggee e não faz rede.
- [x] Debug funciona a partir do DMG montado (somente leitura), sem quarentena bloquear.
- [x] DMG antes/depois medido pelo mesmo método; tamanho registrado na evidência.
- [x] csharp-ls, o plano revisável e o fluxo de análise não mudam.
