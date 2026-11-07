# Evidência — download do netcoredbg limitado por inatividade (issue #162)

Alvo: macOS Apple Silicon (arm64). Spec: [`docs/SPEC-DEBUGGER-DOWNLOAD-STALL.md`](../SPEC-DEBUGGER-DOWNLOAD-STALL.md). Tickets: #178, #179. Política herdada da #161 ([SPEC-TOOLS-DOWNLOAD-STALL](../SPEC-TOOLS-DOWNLOAD-STALL.md)).

O caminho do download só existe **fora** de um `.app`: a build distribuída embute o netcoredbg e `begin_install` recusa instalar quando há adaptador embutido. Toda a prova abaixo é no host real (binário de teste do host e ponte de dev), não no DMG.

## 1. Testes: falha-antes e passa-depois medidos

Testes em `crates/host/src/debugger.rs` (`mod tests`), folhas isoladas (`run_isolated_leaf`) contra um `TcpListener` em `127.0.0.1:0`; o probe (`OPENRIDR_ENABLE_PROBE=1`) é o que libera loopback na allowlist e na guarda de endereço. A janela de stall é parâmetro de `download_with` — o produto passa `http::DOWNLOAD_STALL` (60 s), o teste passa 1 s.

| Caso | Antes (prazo total de 1 s) | Depois (inatividade de 1 s) |
|---|---|---|
| 15 pedaços de 1 KiB, 200 ms entre eles (~3 s) | morre em ~1,25 s com **5.120 de 15.360 bytes** | chega aos **15.360 bytes**, SHA-256 confere |
| Servidor manda 3 de 1.000 bytes e cala | `The download was interrupted: timed out reading response; nothing was extracted.` | `Could not download netcoredbg: … the connection stalled — no data arrived for 1 seconds. Nothing was installed or replaced; check the network and retry the setup.` |
| Servidor aceita e não responde | texto do transporte (`describe_error`) | a mesma frase do app |
| Leitura que falha por outro motivo (`UnexpectedEof`) | — | frase antiga (`The download was interrupted: …`), sem "stalled" |
| Frase do produto | — | `download_stalled(url, DOWNLOAD_STALL)` diz **60 seconds** |

Comandos:

```sh
cargo test -p openridr-host --lib debugger::          # 16 passam, 5 ignorados (as folhas rodam em processo próprio)
cargo test -p openridr-host --lib http::              # 23 passam (a política compartilhada não regrediu)
cargo test -p openridr-host --lib tools::             # 21 passam
cargo test -p openridr-host --test debugger           # 15 passam, 14 ignorados (wrappers das folhas)
```

A falha-antes de cada metade foi medida com o patch oposto, não deduzida:

- **Prazo total**: `download_with` montando o cliente com `.timeout(stall)` (a forma antiga, com a janela do teste como prazo total) reprova o caso lento com 5.120 bytes; com `http::download_agent` passa com 15.360.
- **Mensagem**: `read_failure` devolvendo sempre a frase antiga reprova o caso de stall com `The download was interrupted: timed out reading response; nothing was extracted.`

## 2. Host real: 306 s de transferência até `ready`

O `debug.install` do host real (`crates/host/tests/debugger.rs`, binário de teste, `Host::new` + `dispatch`, com o seam de artefato controlado) rodou contra um servidor local que entrega o zip em 66 pedaços de 5 s. O prazo total que a issue mediu era de **300 s**: a transferência passou dele e terminou.

```
+  0s state downloading
+  5s downloaded 6
…
+300s downloaded 366
+305s state installing
+305s state verifying
+306s state ready
+306s error Null
controlled netcoredbg install: states ["downloading", "installing", "verifying", "ready"],
  verifiedVersion "3.2.0", sha256 "cf028bf7…", server requests 1
```

306 s — 6 s além do prazo de 300 s — com o progresso medido subindo e a instalação em `ready`. O servidor lento é um patch **local, não commitado**, no `Route::Slow` do harness (o teste commitado entrega o corpo de uma vez); o resto é o caminho de produto.

## 3. Host real: stall de 60 s com a frase do app

O mesmo caminho, com o servidor mandando 100 bytes e calando:

```
+  0s state downloading
+  0s downloaded 100
+ 60s state failed
+ 60s error String("Could not download netcoredbg: http://127.0.0.1:51341/asset.zip: the connection stalled — no data arrived for 60 seconds. Nothing was installed or replaced; check the network and retry the setup.")
```

60 s é a janela do **produto** (`DOWNLOAD_STALL`), não a do teste: o stall real do host real falha com a frase do app em `debug.error` e no log, e não com o texto do transporte.

## 4. Não verificado no app: o painel de Debug está atrás do gate

Não houve prova de tela, e o motivo é estrutural, medido no host:

- `setup.state` só vira `ready` com `debug.adapterState == "ready"` (`is_setup_ready`), e em `tauri dev` o adaptador não existe — é o que o download instala.
- `debug.install` exige `require_trust`, que também exige `is_setup_ready`; `trust.grant` com o setup incompleto responde `setup_required` (medido na ponte de dev: `{"error":{"code":"setup_required","message":"Trust requires the verified tool setup; finish or retry the project setup first."}}`).
- Com adaptador embutido o setup fica pronto, mas `begin_install` recusa (`bundled_refusal`).

Resultado: em `tauri dev` o download do netcoredbg não é alcançável nem pela UI nem pela ponte de dev. Na release o adaptador é embutido e não há download. É limite do shell de dev, não defeito desta frente — mas significa que a frase do stall, embora chegue a `debug.error` e ao log, não tem hoje uma tela alcançável onde apareça. Fica registrado em vez de fingir prova de UI.

## 5. Bateria completa

`cargo test -p openridr-host` (uma vez, no fim): lib 423 ok / 14 ignoradas, bundled_analysis 11 ok / 11, discovery 1 ok / 1, debugger 15 ok / 14; `--test host` **107 ok, 1 falha conhecida** — `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build` devolveu `restore_required` com `obj/project.assets.json is missing` sob carga, a mesma assinatura registrada em [`test-stability.md`](test-stability.md) (`build_start_needs_the_current_evaluation… / run_start_reports_restore_required…`). Isolado: **passa** (1 ok em 4,68 s). Não é regressão desta frente, que não toca restore, mtime nem `build.start`.

`bunx tsc --noEmit` verde; `bun test tests/unit` 89 passam; `cargo fmt --all --check` e `cargo clippy --workspace --all-targets -- -D warnings` verdes.

## 6. Limites conhecidos

- **Resolução de nome** continua não interrompível pelo `ureq`; o prazo total anterior também não a limitava. Limite do host inteiro, herdado da #161.
- **Sem prazo total**: um servidor que pinga um byte a cada 59 s mantém o download vivo; o teto que existe é `MAX_DOWNLOAD_BYTES` (20 MiB). Custo aceito na #161 e mantido.
- **A frase diz "retry the setup"**, e o botão do painel de Debug se chama "Retry adapter install". Reusar a frase da #161 foi decisão do grill (uma frase só); o descompasso fica escrito.
- **A URL assinada do asset** saiu das mensagens de erro desta função (`loggable_url` também nos dois `describe_error`), mas `describe_error` de outros caminhos do host continua recebendo a URL crua — não foi auditado aqui.
