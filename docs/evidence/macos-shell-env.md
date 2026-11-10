# Evidência — ambiente do shell de login no macOS (#362)

Máquina desta sessão: **Linux ARM64** (o alvo do bug é macOS ARM64). O parser, a mesclagem, o prazo e
a decisão por plataforma/pai são testados aqui; o caminho Finder → `dotnet restore` de um feed privado
**não** rodou nesta máquina e está no fim como teste manual.

## 1. O que foi feito

`crates/host/src/shell_env.rs` + `shell_env_test.rs`, chamado uma vez por
`src-tauri/src/main.rs::main` antes de `Host::new()`:

- `should_resolve(macos, parent)`: só no macOS e só quando o pai é o pid 1 (`launchd`) — Finder, Dock,
  Spotlight e `open`. Lançamento de terminal tem o shell como pai e não paga nada.
- `resolve(shell, timeout)`: `$SHELL` (fallback `/bin/zsh`) com `-ilc`, stdin em `/dev/null`, stderr
  descartado, stdout lido em thread própria com prazo de 5 s; o script imprime
  `printf '%s' <marcador>; env -0; printf '%s' <marcador>`.
- `parse`: entradas `KEY=VALUE` separadas por NUL entre os marcadores; ruído antes/depois descartado,
  valor com `=` e com quebra de linha preservado, entrada sem chave ou não-UTF-8 descartada.
- `merge`: aplica tudo menos `_`, `SHLVL`, `PWD` e `OLDPWD` (descrevem o shell auxiliar).
- Log: `openridr: shell environment: merged N variable(s) from <shell>` ou o motivo da falha. Nenhum
  valor, em nenhum caminho.

## 2. Testes de host

```
$ cargo test -p openridr-host --lib -- shell_env
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 502 filtered out
```

Os 15 cobrem: marcadores com banner antes e ruído depois; `=` dentro do valor; valor com quebra de
linha; valor vazio; entrada sem chave e sem `=`; entrada não-UTF-8; falta do marcador final; a lista
`NOT_MERGED`; `should_resolve` nas três combinações; `SHELL` ausente/vazio; captura de um filho real;
prazo com filho que não termina; neto segurando o pipe; `resolve` com `/bin/sh` de verdade; `resolve`
com um shell falso que imprime banner antes de `exec /bin/sh "$@"`; shell inexistente; shell que não
imprime ambiente (`/usr/bin/true`) nomeado como tal; e um shell que imprime um segredo **sem**
marcadores, cuja mensagem de falha não repete o que ele imprimiu.

Suíte inteira (o `LC_ALL=C` é do ambiente, não do diff — sem ele três testes de `git::tests` falham
porque o `git` desta máquina responde em português):

```
$ LC_ALL=C LANG=C cargo test -p openridr-host
lib                    503 passed; 0 failed; 14 ignored
bundled_analysis        10 passed; 0 failed; 10 ignored
corporate_proxy          2 passed; 0 failed;  2 ignored
debugger                12 passed; 0 failed; 11 ignored
host                    91 passed; 1 failed; 21 ignored   ← pré-existente, ver §4
test_discovery           7 passed; 0 failed;  0 ignored
test_run                 3 passed; 0 failed;  0 ignored
trust_model              1 passed; 0 failed;  1 ignored
$ cargo clippy -p openridr-host --lib -- -D warnings
Finished `dev` profile
```

`cargo clippy --workspace --all-targets -- -D warnings` no Linux acusa dead code **pré-existente** nos
testes de fronteira (`wait_for_dap`, `write_netcoredbg`, `wait_for_run`, …), que só existem sob
`#[cfg(target_os = "macos")]`; o CI roda esse job no `macos-15`.

## 3. Falha-antes (medida, revertida depois)

Quatro mutações na implementação, uma execução:

| Mutação | Teste que ficou vermelho |
|---|---|
| `should_resolve` sem a condição de macOS | `resolving_is_macos_only_and_only_when_launchd_started_the_app` |
| `merge` sem a lista `NOT_MERGED` | `merging_skips_the_variables_that_describe_the_helper_shell` |
| `parse` sem procurar o marcador final | `the_parser_refuses_output_without_both_markers` |
| `capture` sem o prazo | `capturing_gives_up_on_a_child_that_outlives_the_timeout` (30 s de espera) e `capturing_does_not_wait_for_a_grandchild_holding_the_pipe` |

Resultado: `FAILED. 10 passed; 5 failed`; com as mutações revertidas, `15 passed`.

## 4. Falha pré-existente medida na árvore limpa

`tools_install_is_real_verified_and_reused` falha igual em `origin/main` (`git stash -u` antes de
rodar): o `dotnet` desta máquina imprime a mensagem de assinatura de pacote com outra caixa e o teste
exige a linha literal `Skipping NuGet package signature verification`. Não tem relação com o diff —
o teste baixa `csharp-ls` de `api.nuget.org` e é do caminho de instalação de ferramenta.

## 5. Não verificado — teste manual no Mac (Helder)

O que esta sessão não pode provar é o fluxo que o bug descreve. Roteiro:

1. `launchctl getenv VSS_NUGET_EXTERNAL_FEED_ENDPOINTS` deve sair **vazio** — a credencial só existe no
   `~/.zshrc` (`export VSS_NUGET_EXTERNAL_FEED_ENDPOINTS='{"<feed>":{"username":"…","clearTextPassword":"…"}}'`,
   ou o token que o `nuget.config` referencia).
2. Instalar o bundle desta branch (`bun run app:install`) e **sair do app** por completo.
3. Abrir pelo **Finder** (duplo clique em `/Applications/OpenRidr.app`), nunca pelo terminal.
4. Abrir a solução que usa o feed privado e rodar **Restore** (painel NuGet). Esperado: restore verde;
   antes desta branch ele falha com 401 no feed privado.
5. Opcional, para ver a linha do log: `log stream --process openridr` durante a abertura — deve
   aparecer `openridr: shell environment: merged N variable(s) from /bin/zsh`, sem nenhum valor.
6. Controle: rodar o mesmo restore pelo terminal (`dotnet restore`) — continua verde, como antes.

Não verificado nesta sessão: macOS ARM64 (sem máquina), o restore real do feed privado, a linha do log
no Console, e Linux/Windows no app empacotado (a mudança é um `should_resolve` falso, coberto por
teste).
