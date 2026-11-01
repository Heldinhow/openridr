# Retro — alfa 3: Debug sem pergunta, trust no aviso e testes isolados

Data: 2026-11-01. Entrega: PR [#6](https://github.com/Heldinhow/openridr/pull/6), pré-release
[v0.3.0-alpha.3](https://github.com/Heldinhow/openridr/releases/tag/v0.3.0-alpha.3).

## O que foi entregue

- **Debug em um clique, sem modal.** O `debug-consent-dialog` saiu. `prepareAndDebug` liga
  `debug.flag` sozinho no macOS arm64, reusa/instala o netcoredbg 3.2.0-1092 só se faltar e chama
  `debug.start`; o host continua buildando implicitamente. A nota `dap-experimental-note` mantém o
  status experimental visível sem bloquear.
- **Trust no próprio motivo de bloqueio.** `build-trust`, `run-trust` e `dap-trust` ("Trust this
  project") concedem e memorizam a confiança com `trust.grant {remember:true}`, sem checkbox.
- **Dados de teste isolados.** E2E já usava `OPENRIDR_HOME` temporário; agora prova
  `app.appHome === e2eHome`. `cargo test` também deixou de escrever no app home real: `app_home()`
  detecta binário de teste e usa um diretório temporário por processo. `host:http` usa
  `.scratch/host-home`.
- **Higiene**: `.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/` e
  `docs/evidence/logs/*` ignorados; release 0.3.0-alpha.3 com DMG arm64 ad-hoc e sha256 nas notas.

## O que funcionou

- Remover `startDebug` e deixar um único caminho (`prepareAndDebug`) para atalho, toolbar e painel
  eliminou três guardas duplicadas que já divergiam.
- Tirar adaptador/flag de `debugDisabledReason` foi a mudança mínima para o clique funcionar: o
  gate deixou de bloquear o que o próprio fluxo resolve, e o host continuou validando tudo.
- O sys para o trust ficou no motivo (`<p>` do painel) em vez de criar outro diálogo; o popover de
  `TrustControls` não mudou.
- Isolar o problema no ponto que todos os consumidores usam (`tools::app_home`) cobriu E2E, testes
  de unidade e de integração de uma vez, sem tocar em cada teste.

## O que foi difícil

- O ticket inicial tratou "testes escrevem no app home real" como problema só do E2E. A revisão
  independente reproduziu o caso real: quem escrevia era `cargo test` (helpers `grant()` em
  `host.rs`, `test_discovery.rs`, `test_run.rs`), não o Playwright. O aceite passou a exigir prova
  de mtime/SHA-256 antes/depois também para o `cargo test`.
- O `aria-disabled` da toolbar Debug executa o fluxo mesmo quando o motivo está visível; hoje isso
  é intencional (mensagem no painel em vez de clique morto), mas é uma semântica diferente dos
  outros botões — documentado no `decisions.md`.
- `trust-error` já existia no popover de `TrustControls`; o erro novo do aviso colidia no mesmo
  testid e foi renomeado para `trust-grant-error`.
- A falha de baseline `file_access_is_confined_to_the_workspace` continua vermelha em paths longos
  de `/var/folders/...`: `../../../../../../etc/hosts` resolve para um arquivo existente e o teste
  espera `outside_workspace`, mas recebe `not_found`. Não é regressão desta entrega.

## Próximos passos / dívida

- Limpar as entradas históricas de pastas temporárias já gravadas em
  `~/Library/Application Support/OpenRidr/trusted.json` (148 entradas, 1 de `/tmp`; o backup
  `trusted.json.bak-2026-11-01` já existe). Não foi feito: é dado do usuário.
- `file_access_is_confined_to_the_workspace` precisa de um caminho de fixture com profundidade
  estável (ou de comparar canonicalização) para voltar a ser um teste confiável.
- Logs já versionados em `docs/evidence/logs/` continuam rastreados; a regra de ignore vale para
  arquivos novos. Migrar os antigos é uma decisão separada.
- Windows x64, macOS Intel e a janela Tauri/WKWebView empacotada não foram executados nesta rodada.
