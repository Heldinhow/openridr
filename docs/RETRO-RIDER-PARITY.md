# Retro: paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Spec: `docs/SPEC-RIDER-PARITY.md`. Evidência:
`docs/evidence/rider-parity.md`.

Esta retro foi escrita à mão. A skill `/retro` não estava disponível nesta sessão.

## Estado

- Tickets 01–12: implementados e commitados localmente em rodadas anteriores.
- Tickets 13–16: adiados, não tocados.
- Aceite no app real (agent-desktop): **não feito**. Bloqueado por uma instância de OpenRidr
  de outro worktree aberta. Nenhum ticket está verificado no app real.
- Playwright dos specs ligados aos tickets: 115 passaram, 2 falharam (falhas conhecidas em
  `run-configs`), 1 pulado (identificado depois como o ambiente de `csharp-ls` na rodada
  completa; ver evidência).
- `find-usages`: o teste de "Start analysis" passou; o teste de referências falhou 2 de 2 com
  `Partial coverage`, não investigado.

## Funcionou

- Permissões do agent-desktop: Acessibilidade e Gravação de Tela foram concedidas pelo Helder
  e passaram a valer sem reiniciar o agente.
- `permissions --request` não abriu diálogo; o caminho que funcionou foi `permissions --guide`
  e a concessão manual em Ajustes do Sistema.
- Build de debug do bundle com `tauri build --debug --bundles app`. Sai em `src-tauri/target`,
  sem tocar `/Applications`.
- Specs de Playwright com porta alternativa (`OPENRIDR_E2E_UI_PORT=1430`,
  `OPENRIDR_E2E_HOST_PORT=4327`) rodam ao lado de outra sessão que usa 1420 e 4317.
- O teste de "Start analysis" de Find Usages prova o caminho do aviso até a análise pronta.

## Não funcionou

- `agent-desktop status` mostrou `denied` mesmo depois de conceder a permissão no macOS.
  O processo pai (T3 Code) precisou ser o app autorizado. Vale checar o status antes de
  planejar o roteiro.
- A primeira execução do Playwright foi interrompida antes do fim e não deixou resultado
  completo. Só a segunda rodada completa conta.
- `desktop.sh up` recusa iniciar se houver qualquer `openridr` rodando, mesmo de outro
  worktree. Isso bloqueou o aceite e não há contorno seguro sem encerrar a instância alheia.
- `find-usages`: `lsp-state` fica em "Partial coverage" em vez de "Ready" no teste de
  referências. O AGENTS.md diz intermitente; aqui foi 2 de 2.

## Lições

- Antes de declarar um aceite de app real, rodar `agent-desktop status` e `pgrep -x openridr`.
  Os dois são baratos e apontam o bloqueio antes do roteiro.
- Não encerrar processo de outra sessão para liberar o caminho. Pedir ao dono.
- Specs que pulam (`skip`) não contam como passe. O reporter de linha não diz qual pulou; use
  `--reporter=list` quando precisar identificar o pulo.
- Evidência que depende de app real deve sair como "sem evidência visual" até o roteiro rodar.
  Rodar Playwright não prova o que o usuário vê.

## Próximos passos

1. Fechar a instância aberta do OpenRidr (decisão do Helder) e rodar o aceite dos tickets 01–12
   com `desktop.sh up src-tauri/target/debug/bundle/macos/OpenRidr.app`.
2. Investigar `find-usages` "Partial coverage" e checar se já ocorria no HEAD anterior.
3. Se o aceite no app real passar, atualizar os tickets e marcar a evidência; se falhar,
   corrigir com teste e um commit local por correção.
