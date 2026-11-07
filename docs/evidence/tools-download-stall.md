# Evidência — download de ferramenta limitado por inatividade (issue #161)

Alvo: macOS Apple Silicon (arm64), OpenRidr 0.3.0-alpha.11 em `tauri dev`. Spec: [`docs/SPEC-TOOLS-DOWNLOAD-STALL.md`](../SPEC-TOOLS-DOWNLOAD-STALL.md). Tickets: #163, #164, #165.

O caminho do download só existe **fora** de um `.app`: a build distribuída embute o csharp-ls e o runtime privado, e o plano de setup recusa baixar quando o par está presente. Toda a prova abaixo é, portanto, no shell de desenvolvimento (`tauri dev`) e no host isolado — não no DMG.

## 1. Host: falha-antes e passa-depois medidos

Testes em `crates/host/src/http.rs` (`mod tests`), contra um proxy local que serve o corpo em pedaços. O destino é um IP público literal e o servidor é o proxy configurado, então não há DNS nem rede externa, e o teste não escreve no ambiente do processo (usa a variante do cliente com ambiente injetado).

| Caso | Antes (prazo total) | Depois (inatividade) |
|---|---|---|
| 15 pedaços de 4 KiB, 100 ms entre eles (~1,5 s), janela de 400 ms | `the whole body arrives: Custom { kind: TimedOut }` — morreu no primeiro pedaço | passa, `bytes.len() == 61440` |
| Servidor que aceita e não responde, janela de 1 s | não terminaria antes dos 120 s do prazo total | volta com a frase do stall |
| Servidor que manda 3 bytes de 1.000 prometidos e cala | `timed out reading response` (texto do transporte) | `is_stall` verdadeiro + a frase do app |

Comandos:

```sh
cargo test -p openridr-host --lib http::tests          # 6 passam
cargo test -p openridr-host --lib                      # 391 passam, 11 ignorados
cargo test -p openridr-host --test host -- tools_ a_download_without   # 5 wrappers passam (as folhas isoladas rodam em processo próprio)
```

A falha-antes da **mensagem** foi medida isoladamente: com a política de inatividade e sem o mapeamento de `TimedOut` para a frase do app, o teste do servidor mudo reprova com o texto cru do `ureq` (`… Network Error: Error encountered in the status line: timed out reading response Transport(Transport { … })`).

## 2. Host isolado: transferência de 146 s completa (ponte HTTP)

Antes de abrir o app, o mesmo host da ponte de dev (`openridr-host-http`, `OPENRIDR_HOME` isolado) rodou o download real contra um servidor local que entrega 15 MiB em 30 pedaços de 512 KiB a cada 5 s:

```
07:59:26 downloading {'downloaded': 524288, 'total': 15728640}
07:59:36 downloading {'downloaded': 1048576, 'total': 15728640}
…
08:01:47 downloading {'downloaded': 15204352, 'total': 15728640}
08:01:52 installing  {'downloaded': 15728640, 'total': 15728640}
08:01:57 verifying
08:02:02 ready
```

146 s de transferência — 26 s além do prazo total de 120 s que a #161 mediu — com o progresso medido subindo e a instalação terminando em `ready`.

## 3. App Tauri real (`agent-desktop`)

Duas execuções de `tauri dev` (o binário fora do `.app`), uma instância por vez, `OPENRIDR_HOME` próprio em `/tmp/verify-openridr-161`, seams de debug (`OPENRIDR_ENABLE_PROBE=1`, `OPENRIDR_TOOLS_MUXER` com um instalador controlado, `OPENRIDR_TOOLS_PACKAGE_URL`/`_SHA512` para o servidor local) e a fixture `OrderDesk` copiada em `/tmp` aberta por argv.

### 3.1 Lento que progride

Servidor em modo lento (15 MiB, 30 pedaços de 512 KiB a cada 5 s). O `GET` foi registrado às `08:03:11`; a tela de setup, lida pela árvore de acessibilidade:

| Hora | Estado na tela |
|---|---|
| `08:03:49` | `Downloading the pinned package` · `Downloaded 4194304 of 15728640 bytes (26%)` |
| `08:05:08` | `Downloaded 12582912 of 15728640 bytes (80%)` |
| `08:05:45` | `Analysis tool · csharp-ls 0.28.0` → `Verified` + caminho e SHA-256 do launcher |

~150 s de transferência, 30 s além do prazo antigo, com o progresso subindo na tela e a verificação no fim. Captura: [`tools-download-stall/slow-progress-26.png`](tools-download-stall/slow-progress-26.png).

### 3.2 Stall, mensagem e retry

Servidor em modo stall (um pedaço de 512 KiB e silêncio). `GET` às `08:06:23`; a tela mostrou `Setup failed` e, na linha do csharp-ls, `Failed`; 60 s depois o texto do app:

```
Could not download csharp-ls: http://127.0.0.1:18761/csharp-ls.0.28.0.nupkg: the connection stalled —
no data arrived for 60 seconds. Nothing was installed or replaced; check the network and retry the setup.
```

`Retry setup` (a ação que o gate oferece) às `08:08:35`, com o servidor de volta ao normal → `GET` registrado, download completo e `Verified`; o launcher instalado responde `csharp-ls, version 0.28.0 (probe)`. Capturas: [`stall-message-and-retry.png`](tools-download-stall/stall-message-and-retry.png), [`stall-retry-verified.png`](tools-download-stall/stall-retry-verified.png).

O `Setup failed` no topo continua por causa do **netcoredbg**: fora de um `.app` o adaptador embutido não existe e a flag experimental está desligada — limitação de desenvolvimento anterior a esta frente, sem relação com o csharp-ls (a linha do csharp-ls mostra `Verified`).

### 3.3 Achado corrigido durante a prova

O gate de setup mostrava só `setup.error`, nunca `tool.error`: um download que falha aparecia como `Failed` **sem motivo**, e o gate é a única superfície enquanto o setup bloqueia o workspace (o painel de ferramentas fica atrás dele). Uma linha em `SetupGate.tsx` passou a mostrar o erro da ferramenta em parágrafo próprio (`data-testid="setup-gate-tool-error"`, testid **novo**; nenhum existente mudou). A mensagem da seção 3.2 foi lida depois dessa correção, com o app ainda aberto (HMR do Vite).

## 4. O que não foi verificado

- **Windows e macOS Intel**: não executados.
- **O download do netcoredbg** (mesma forma, prazo total de 300 s): fora do escopo desta frente, na #162.
- **Cancelar o download pela UI**: o host tem `tools.cancel`, a UI não o expõe; não exercitado.
- **Um stall de 60 s no meio de um download grande de verdade** (NuGet.org): a prova usou um servidor local; a rede real não foi medida.
- **A build distribuída**: o caminho do download não existe nela por desenho, então não há o que provar ali.

## 5. Reprodução

O harness da prova ficou fora do repositório, em `/tmp/verify-openridr-161`: `server.py` (modos `slow`/`stall`/`fast` lidos de um arquivo), `installer` (instalador controlado que escreve o launcher `0.28.0`), `payload.sha512` (SHA-512 dos 15 MiB de `x`) e `run-app.sh` (as variáveis de ambiente do `tauri dev`). Nada disso entra no repositório: são seams de debug, não produto.
