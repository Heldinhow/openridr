# Evidência da release 0.3.0-alpha.10

Base `4758157068ff3faf6fd05e4b92d0bd220812d8cb`, macOS ARM64, com bump de versão e lockfile
Tauri atualizado para incluir as dependências TLS já introduzidas no #38.

## Assinatura e notarização

- App: Developer ID Application, hardened runtime; `codesign --verify --deep --strict` passou.
- Apple aceitou o app (`c7328b8b-8dff-406d-ba0d-614d3dd5448d`) e o DMG
  (`69f73d3f-7711-427e-a5f9-a5a2d805d1eb`); os dois tickets foram grampeados e validados.
- `spctl` aceitou app e DMG como `Notarized Developer ID`. O app montado passou em `codesign` e
  `spctl`, com `CFBundleShortVersionString` 0.3.0-alpha.10.
- DMG final: 69.623.227 bytes; SHA-256
  `b106b31672ec387cfbd4fea02cb396fde8a0d4d69dd394eecd72772cac15ece3`.

## Smoke desktop

Uma cópia do DMG recebeu `com.apple.quarantine`, foi montada e seu app copiado e aberto com uma
cópia da fixture `OrderDesk` e `OPENRIDR_HOME` vazio e isolado. O processo executou em
`AppTranslocation`. Não foi observado diálogo gráfico de confirmação nesta execução; isso não é
prova do comportamento do diálogo no primeiro download em outra máquina.

Com uma única instância, o primeiro passo visível foi Trust, sem diálogo de download; o home não
continha `tools/` e o processo do `csharp-ls` vinha de `Contents/Resources` do app translocado.
O build do projeto inicial mostrou `Build succeeded · exit 0`, 0 warnings e 0 errors. Após inserir
`int broken = "not an int";` no editor, Problems mostrou `error CS0029`, origem LSP/csharp-ls e
cobertura parcial. A espera automatizada por `statictext:CS0029` expirou: o diagnóstico era um
**botão**, confirmado por snapshot posterior, não uma ausência de diagnóstico. O app foi fechado e
o volume desmontado. Interface dirigida apenas com agent-desktop.

## Testes e limites

- `bun run check`: passou; `bun run build`: passou durante o build Tauri.
- `bun test tests/unit`: 35 passaram, 0 falharam.
- `cargo test -p openridr-host`: 408 passaram, 0 falharam, 0 ignorados (270 + 8 + 14 + 98 + 10 + 7 + 1).
- Não executados: E2E/Playwright, Intel, Windows e teste de HTTPS com CA corporativa real.
- O artefato publicado e o download de volta ainda não foram verificados: dependem do merge e da
  publicação pelo fluxo de release.
