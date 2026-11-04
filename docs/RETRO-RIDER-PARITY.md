# Retro: paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Spec: `docs/SPEC-RIDER-PARITY.md`. Evidência:
`docs/evidence/rider-parity.md`.

Esta retro foi escrita à mão. A skill `/retro` não estava disponível nesta sessão.

## Estado

- Tickets 01–12: implementados e commitados localmente em rodadas anteriores.
- Tickets 13–16: adiados, não tocados.
- App real (agent-desktop): rodou com o bundle de debug do checkout. Provado em 01, 02, 03, 04,
  10 e 12 (ver a tabela de evidência). Ficou sem evidência visual o que depende de teclado,
  digitação, clique na margem ou hover.
- Playwright dos specs ligados aos tickets: 115 passaram, 2 falharam (falhas conhecidas em
  `run-configs`), 1 pulado e ainda não identificado.
- `find-usages` "lists the real references": falha **já no `main`**, com a mesma asserção.
  Não é regressão desta branch.

## Funcionou

- Permissões do agent-desktop: Acessibilidade e Gravação de Tela concedidas pelo Helder. A
  Acessibilidade passou a valer depois da concessão, sem reiniciar o agente. A Gravação de Tela
  só passou a valer numa nova checagem.
- `permissions --guide` abriu o painel certo; `permissions --request` não abriu diálogo.
- Build de debug com `tauri build --debug --bundles app`, sem tocar `/Applications`.
- O fluxo de "Trust project" e "Run all" funcionou headless (ações semânticas).
- Árvore de acessibilidade e texto de tela mostraram o que a evidência precisava: perfil de
  keymap, plano de instalação, contagem de testes, "Ln 29, Col 1".
- Comparação com `main` em worktree temporário, com home de E2E separado, deu resposta
  definitiva sobre a falha de `find-usages`.

## Não funcionou

- `agent-desktop status` mostrou `denied` mesmo depois de conceder a permissão no macOS, até
  o app ser reaberto. Vale checar o status antes de planejar o roteiro.
- `desktop.sh up` recusou iniciar porque havia outro `openridr` rodando, de outro worktree.
  Bloqueou o aceite até o Helder encerrar a instância.
- A primeira execução do Playwright foi interrompida antes do fim e não deixou resultado.
- Itens sem ref semântico: os disclosures "Details" dos resultados de teste e o combobox da
  configuração de execução não podem ser clicados sem coordenadas.
- O menu de pasta do OpenRidr não apareceu na árvore de acessibilidade. O clique direito
  semântico abriu o menu padrão do WebKit. Não investiguei se o handler está no DOM.
- `press` envia tecla por CGEventPost (fisicamente, ao app em foco), mesmo sem `--headed` no
  nome do comando. Foi usado uma vez, para fechar aquele menu do WebKit, e não deve ser
  tratado como ação semântica.

## Lições

- Antes de declarar um aceite de app real, rodar `agent-desktop status` e `pgrep -x openridr`.
  Os dois são baratos e apontam o bloqueio antes do roteiro.
- Não encerrar processo de outra sessão para liberar o caminho. Pedir ao dono.
- Specs que pulam (`skip`) não contam como passe. O reporter de linha não diz qual pulou; use
  `--reporter=list` para identificar.
- Quando uma falha da lista de conhecidas aparecer, comparar com `main` antes de atribuir à
  branch. Foi o que separou a falha de `find-usages` de uma regressão.
- Evidência de app real que depende de teclado ou coordenadas fica "sem evidência visual" até
  haver permissão explícita para `--headed`. Rodar Playwright não prova o que o usuário vê.
- Dados pessoais e caminhos locais saem das evidências com `sed` (home → `~`, temporários →
  `<tmp>`) e uma varredura final com `grep`.

## Próximos passos

1. Decidir se o Helder autoriza `--headed` (cursor e foco do Mac) para fechar os atalhos,
   digitação, clique na margem e hover dos tickets 02, 06, 07, 08, 11 e 12.
2. Instalar `csharp-ls` no home do app isolado (download NuGet, precisa de autorização) para
   rodar o "Start analysis" do ticket 01 no app.
3. Investigar a falha de `find-usages` "Partial coverage" no `main`. Ela existe antes desta
   branch e não é deste trabalho.
4. Identificar o teste pulado da rodada completa de Playwright.
