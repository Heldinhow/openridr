# Evidência — proxy do macOS System Settings (#132; tickets #181, #182)

Máquina: macOS ARM64 (Darwin 25.x), sem proxy configurado no System Settings — o que é justamente o
caso que permite provar a leitura real sem alterar a configuração de rede do usuário. Rede externa
usada: nenhuma nos testes; o destino dos testes é IP público literal (`1.1.1.1`) e o proxy é um
`TcpListener` local que responde 502.

## 1. A leitura real do ajuste

```
$ scutil --proxy
<dictionary> {
  ExceptionsList : <array> {
    0 : *.local
    1 : 169.254/16
  }
  FTPPassive : 1
}
$ time scutil --proxy > /dev/null
real 0.01
```

Sem proxy configurado, o dicionário traz só `ExceptionsList`/`FTPPassive`: a rota continua direta,
como antes do #132. Essa amostra é o fixture `NO_PROXY` de `crates/host/src/system_proxy_test.rs`.

## 2. Testes de host

```
$ cargo test -p openridr-host --lib -- system_proxy
test result: ok. 17 passed; 0 failed; 0 ignored
$ cargo test -p openridr-host --lib -- http
test result: ok. 40 passed; 0 failed; 0 ignored
$ cargo test -p openridr-host --test corporate_proxy
test result: ok. 2 passed; 0 failed; 2 ignored
```

O leaf isolado (`discover_through_the_system_proxy_reports_the_proxy_isolated`) roda com `dotnet` no
PATH: o host recebe `OPENRIDR_SYSTEM_PROXY` apontando para um `scutil` falso que imprime
`HTTPSProxy : 127.0.0.1` + a porta do proxy local, e o Discover do NuGet falha nomeando
`127.0.0.1:<porta>` com o proxy registrando `CONNECT 1.1.1.1:443`. É a prova de que o caminho de
produto (`nuget.search` → `http::agent_for` → fonte do sistema) usa o ajuste.

## 3. Falha-antes (medida, revertida depois)

| Teste | Variante sem a mudança | Resultado |
|---|---|---|
| `a_proxy_from_the_system_settings_is_used_when_the_environment_is_empty` | `plan(..., None)` | FAILED (`left: None`) |
| `the_system_exceptions_send_the_destination_direct` | `plan(..., None)` | FAILED |
| leaf `discover_through_the_system_proxy_reports_the_proxy_isolated` | seam em `off` | FAILED (0 hits no proxy) |
| `the_system_proxy_label_names_its_origin` | `described()` = netloc nu | FAILED (`left: Some("proxy.corp:3128")`) |
| `the_download_log_names_the_system_proxy_and_its_origin` | `described()` = netloc nu | FAILED |
| `a_failure_with_socks_names_the_setting_and_the_way_out` | `describe_via` ignora `unsupported` | FAILED |
| `a_failure_with_a_pac_file_names_the_setting` | `describe_via` ignora `unsupported` | FAILED |

`an_unusable_system_proxy_names_the_system_settings` passa nos dois lados: a mensagem de falha
fechada já nomeia a origem desde o #181.

## 4. App real (agent-desktop)

A preencher: prova no app Tauri de debug com o seam de debug apontando para um `scutil` falso e um
proxy local, mais o "não verificado" do proxy realmente configurado no System Settings.
