# Evidência da release 0.3.0-alpha.11

Base `6c278aa0943ec6026ce076d9d7b08ae2ea0e50fb`, macOS ARM64, com bump de versão.

## Assinatura e notarização

- App Developer ID com hardened runtime; runtime .NET e netcoredbg assinados com seus entitlements.
- Apple aceitou o app (`ce97f555-0241-4f3d-8976-8e8e81b153ea`) e o DMG
  (`072c73f0-b789-41f3-be5e-570a362baf85`); ambos foram grampeados e validados.
- `codesign --verify --deep --strict` e `spctl` passaram no app, inclusive montado; `spctl` aceitou
  o DMG. A versão montada é `0.3.0-alpha.11`.
- DMG: 69.643.243 bytes; SHA-256
  `15bf37e8355127796ac0cf03b2c767f31495275ebc9847d623dd60c027880510`.

## Smoke desktop

Uma cópia do DMG recebeu quarentena, foi montada e o app copiado abriu com `OPENRIDR_HOME` vazio e
isolado. A interface foi dirigida somente com agent-desktop. Não foi observado diálogo gráfico de
primeira abertura, nem verificado o caminho de translocação. A abertura inicial chegou à tela de
boas-vindas sem download; o home isolado não tinha diretório `tools/`.

O diálogo nativo de abertura não respondeu à automação de acessibilidade; o app foi reiniciado com
uma cópia de `LaunchProfiles.csproj` em `/tmp` como argumento. Após confiar no projeto, o seletor
mostrou `http`, `https` e `IIS Express` não suportado. Run `http` terminou com exit 0 e imprimiu
`ASPNETCORE_URLS=http://localhost:5083`; Build terminou com exit 0, sem warnings ou erros. Inserir
um erro no editor fez Problems mostrar `CS0029`, origem LSP/csharp-ls. A alteração foi descartada,
o app fechado e o volume desmontado.

## Testes e limites

- `bun run check` passou; `bun test tests/unit`: 35 passaram.
- `cargo test -p openridr-host`: bibliotecas 283/283, bundled 8/8 e debugger 14/14; em `host`,
  97/98 passaram. `keep_leaves_the_process_running_and_terminate_escalates_after_the_grace_period`
  falhou porque `OPENRIDR_ENABLE_PROBE` estava desabilitado durante execução concorrente; passou
  isoladamente com uma thread. As suítes que não rodaram após essa falha passaram separadamente:
  `test_discovery` 10/10, `test_run` 7/7 e `trust_model` 1/1.
- Não executados: E2E/Playwright, Intel e Windows. Publicação e download de volta dependem do merge.
