# Evidência — o snapshot pergunta ao muxer qual SDK ele resolve (issue #310)

Data: 2026-11-08. macOS ARM64 (Apple Silicon), `~/.dotnet` com SDK **8.0.408** e **10.0.403**
(nenhum prerelease instalado). Branch `fix/issue-310-muxer-sdk`, base `c584c6e`.

## Reprodução medida à mão (antes de qualquer código)

`/tmp/X/global.json` = `{ "sdk": { "version": "8.0.408", "rollForward": "disable" } }`; projeto
`/tmp/X/app/App.csproj` com `net10.0` e **sem** `global.json` próprio. Todos os comandos com
`DOTNET_ROOT=$HOME/.dotnet` e `DOTNET_MULTILEVEL_LOOKUP=0`:

| Comando | Resultado |
|---|---|
| `dotnet --version` com cwd `/tmp/X/app` e o `global.json` acima | `8.0.408` |
| o mesmo, depois de remover `/tmp/X/global.json` | `10.0.403` |
| `dotnet msbuild App.csproj -getProperty:TargetPath` com o pin acima | **sucesso** (`bin/Debug/net10.0/App.dll`) |
| `dotnet build App.csproj` com o pin acima | **falha** `NETSDK1045: The current .NET SDK does not support targeting .NET 10.0` |

Ou seja: a tela dizia `10.0.403` (a regra do host), o muxer rodava `8.0.408`, a avaliação passava e o
build quebrava com `state == "resolved"` — as duas metades do problema da issue.

## O teste vermelho (a reprodução no host)

`crates/host/tests/host.rs::the_snapshot_names_the_sdk_the_muxer_resolves_not_its_own_rule` e
`a_muxer_pin_that_misses_the_requirement_is_sdk_missing`: um `tempdir` com `global.json` na raiz e a
workspace em `app/`, aberta no `.csproj`, para que o host (que só lê `<raiz da workspace>/global.json`)
não veja o pin e o muxer (que roda em `app/`) veja. As versões saem de `sdk::inventory()`, nunca
hardcoded; sem duas linhas estáveis ≥ 5 na mesma raiz os testes imprimem `not verified:` e retornam.

Antes da mudança, com `git stash push -- crates/host/src`:

```
test a_muxer_pin_that_misses_the_requirement_is_sdk_missing ... FAILED
test the_snapshot_names_the_sdk_the_muxer_resolves_not_its_own_rule ... FAILED
  assertion `left == right` failed: the muxer reads the ancestor pin; the host rule would name 10.0.403
    left: String("10.0.403")
   right: "8.0.408"
  assertion `left == right` failed
    left: String("resolved")
   right: "sdk_missing"
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 144 filtered out
```

Depois da mudança:

```
$ cargo test -p openridr-host --test host -- muxer
test a_muxer_pin_that_misses_the_requirement_is_sdk_missing ... ok
test the_snapshot_names_the_sdk_the_muxer_resolves_not_its_own_rule ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 144 filtered out; finished in 0.22s
```

## O resto do que rodou

| Comando | Resultado |
|---|---|
| `cargo test -p openridr-host --lib -- sdk::` | 30 passed, 0 failed |
| `cargo test -p openridr-host --test host -- sdk` | 12 passed, 0 failed, 3 ignored (os leaves isolados de sempre) |
| `cargo test -p openridr-host` (suíte inteira, o job `rust` do CI) | ver o rodapé deste arquivo |
| `cargo fmt --all -- --check` | limpo |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpo |
| `PLAYWRIGHT_CHANNEL=chromium OPENRIDR_E2E_UI_PORT=2320 OPENRIDR_E2E_HOST_PORT=5317 bunx playwright test tests/e2e/sdk-prerequisites.spec.ts` | **7 passed** (17,9 s) |

O spec de E2E é o do #289 e não mudou: ele prova que a UI renderizada continua dizendo o SDK que o
muxer resolve no caso comum (`fixtures/Net10NoPin` → 10.0.403) e que os cenários ausente/incompatível
continuam iguais. Chrome não está instalado nesta máquina, por isso `PLAYWRIGHT_CHANNEL=chromium`.

## Não verificado

- **Prerelease mais novo que o maior estável (caso B da issue).** Não há prerelease instalado nesta
  máquina e instalar um é decisão do usuário, não do app. A mudança cobre o caso por construção — a
  versão reportada é a que o muxer responde, seja ela qual for — mas **não foi observada**.
- **Os caminhos de erro de `muxer_version`** (muxer que não inicia, exit não zero, saída que não é uma
  versão): sem teste, porque o teste exigiria um muxer falso. O caminho de sucesso é o exercitado pelos
  dois testes de fronteira.
- **`global.json` entre a raiz da workspace e o diretório do projeto.** O muxer é consultado na raiz
  (o `cwd` de restore/build/run/test) e a avaliação roda no diretório do projeto; a divergência que
  sobra está registrada como `shortcut:` em `resolve_without_pin`.
- **Windows e macOS Intel**, como no resto do produto.
