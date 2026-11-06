# Perfis de launchSettings — evidência

Data: 2026-11-06. macOS ARM64, bundle debug `0.3.0-alpha.10` construído neste worktree,
SDK de projeto `8.0.408`, app home e cópia da fixture isolados em `<tmp>`. Uma instância do
app; interação e capturas somente com `agent-desktop`.

## App real

- [Seletor](launch-settings/01-profiles.png): primeiro perfil `Project` selecionado por padrão;
  `http` e `https` importados, `IIS Express` desabilitado com motivo.
- [Run https](launch-settings/02-https-run.png): saída `PROFILE_ENV=https` e
  `ASPNETCORE_URLS=https://localhost:7259;http://localhost:5083`; processo terminou com exit 0.
- [Run http](launch-settings/03-http-run.png): saída `PROFILE_ENV=http`, URL HTTP,
  `ARGS=--message|hello world` e diretório da cópia do projeto; exit 0.
- [Debug http](launch-settings/04-http-debug.png): console do debuggee mostra as mesmas variáveis,
  argumentos e diretório. O adaptador macOS ARM64 é experimental.
- [JSON inválido](launch-settings/05-invalid.png): erro inclui o arquivo e motivo; seletor cai
  na configuração padrão. [Recuperação](launch-settings/06-reloaded.png): restaurar o arquivo
  e reabrir o seletor relê os perfis e limpa o aviso anterior.
- O arquivo de configurações no app home isolado continha apenas o nome escolhido por projeto;
  a lista de configurações manuais permaneceu vazia.

## Testes

- `cargo test -p openridr-host launch_profile --lib`: 3 passaram, incluindo execução real da
  fixture copiada com env, URL, argumentos e diretório efetivos.
- `cargo test -p openridr-host manual_config_writes_preserve_per_project_profile_choices --lib`:
  passou.
- `TMPDIR=/tmp cargo test -p openridr-host`: 273 testes de biblioteca passaram e um falhou
  porque `/tmp/openridr-neutral` não existia. Após criar o diretório, o teste falho passou
  isoladamente. Os seis binários de integração foram rodados separadamente: 138 passaram.
- `bun run check`: passou; `bun run test:unit`: 35 passaram.
- `bun run tauri build --debug --bundles app`: passou; bundle assinado ad hoc.

Não executados: Playwright (explicitamente proibido nesta tarefa), Windows e macOS Intel.
`launchBrowser` e `launchUrl` permanecem fora do escopo. A comparação de compatibilidade de
todos os casos de escape de `commandLineArgs` com o CLI .NET não foi realizada.
