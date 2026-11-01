# Retro — build implícito em Run/Debug (0.3.0-alpha.2)

Data: 2026-11-01. Entrega: PR [#5](https://github.com/Heldinhow/openridr/pull/5), pré-release
[v0.3.0-alpha.2](https://github.com/Heldinhow/openridr/releases/tag/v0.3.0-alpha.2).

## O que foi entregue

- Run/Debug iniciam o build normal quando não há build atual e só lançam com exit 0 observado;
  build atual executa direto, sem rebuild.
- Estados `building` no host/UI, cancelamento encadeado (`run.cancel`, `debug.stop`,
  `build.cancel`, fronteiras) e recusa de segundo lançamento enquanto o build está pendente.
- Restore continua explícito; `restore_required` e a ação de Restore não mudaram.
- Testes host de sucesso/falha/cancelamento para Run e Debug, gates/E2E atualizados e evidência
  real com `fixtures/OrderDesk` sem build em `docs/evidence/auto-build-run-debug.md`.

## O que funcionou

- Reutilizar `build.start` como pipeline único evitou um segundo caminho de compilação e fez o
  painel Build, o log e os diagnósticos aparecerem de graça.
- Registros pendentes separados do processo (`pending_run`/`pending_debug`) deixaram o
  cancelamento e a conclusão do build explícitos, sem inventar `exit`/`termination`.
- O review independente pegou uma corrida real: `check_external_changes` rodava depois do
  lançamento pendente. A correção foi só de ordem, mas o teste determinístico da janela não é
  viável no harness atual.
- Rodar os E2E com dotnet e netcoredbg reais confirmou o fluxo completo (Run sem build e Debug
  sem build parando em breakpoint), não só o contrato do host.

## O que foi difícil

- O `snapshot` por polling é o ponto onde build, run e DAP terminam; a ordem entre observar o
  build, invalidar avaliação e lançar precisou ser explícita.
- Reutilizar/queimar a sessão DAP ao cancelar um Debug pendente exigiu guardar a sessão na
  pendência; sem isso, um cancelar e reiniciar repetia o número de sessão.
- A UI tinha três caminhos antigos de preparo (toolbar, atalho, painéis). Remover
  `prepareAndRun`/`prepareBuild` do Run e deixar o host como única fonte de verdade reduziu o
  diff, mas exigiu atualizar gates e textos em vários specs.

## Próximos passos / dívida

- Teste determinístico da corrida entre mudança externa de `.csproj` e conclusão do build
  continua ausente; hoje a garantia é a ordem no `snapshot` + gates do host.
- Windows x64, macOS Intel e a janela Tauri/WKWebView nativa não foram executados nesta rodada.
- O Debug real é `skip` quando o netcoredbg 3.2.0-1092 não está no app home padrão.
- A tag `v0.3.0-alpha.2` aponta para o commit de merge (85e7074); o SHA-256 das notas foi
  commitado depois do merge (4b3c9d6), então o arquivo dentro da tag ainda mostra o placeholder.
  O release publicado usa o arquivo atualizado.
