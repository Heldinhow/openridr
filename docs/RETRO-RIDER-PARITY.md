# Retro: paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Spec: `docs/SPEC-RIDER-PARITY.md`. Evidência:
`docs/evidence/rider-parity.md`.

Esta retro foi escrita à mão. A skill `/retro` não estava disponível nesta sessão.

## Estado

- Tickets 01–12: implementados e commitados localmente em rodadas anteriores. Nenhum defeito
  novo apareceu no aceite, então não houve correção de código nesta rodada.
- Tickets 13–16: adiados, não tocados.
- App real (agent-desktop, `--headed` autorizado): provado em 01, 02, 03, 04, 05, 07, 08, 10 e
  12, com ressalvas por item na evidência. Sem evidência visual: 06, 09 (Duplicate to edit), 11,
  tooltips do 12, duplo Shift e "Run Unit Test".
- Playwright dos specs ligados aos tickets: 115 passaram, 2 falharam (falhas conhecidas em
  `run-configs`), 1 pulado e ainda não identificado.
- `find-usages` "lists the real references": falha **já no `main`**, com a mesma asserção.
  Não é regressão desta branch.

## Funcionou

- Permissões do agent-desktop concedidas pelo Helder. Acessibilidade valeu depois da concessão;
  Gravação de Tela só passou a valer numa nova checagem.
- `desktop.sh up` com o bundle de debug, depois que a instância bloqueadora foi encerrada.
- Instalação do `csharp-ls` pelo próprio app, com confirmação e verificação, no home isolado.
- ⇧F12 com teclado real listou as referências; ⌘T, ⌘1 e ⌘8 alternaram o que deviam.
- Menu da pasta e Show in Finder com o item selecionado, provados no app.
- Comparação com `main` em worktree temporário com home de E2E separado: resposta definitiva
  sobre a falha de `find-usages`.

## Não funcionou

- `agent-desktop status` mostrou `denied` até a permissão valer para o processo que roda o
  agente. Checar o status antes de planejar o roteiro.
- `desktop.sh up` recusou iniciar porque havia outro `openridr`, de outro worktree.
- Clique direito semântico na pasta abriu o menu padrão do WebKit. O menu do OpenRidr só apareceu
  com `--headed` num diretório real (`bin · no index`); a pasta de solução não abre o menu.
- Duplo Shift não disparou com eventos de modificador isolados (`press`, `key-down`/`key-up`).
- Enter no campo Find não avançou o contador.
- Tooltip do botão Monitoring não apareceu na captura com hover.
- Itens sem ref semântico: disclosures "Details" dos resultados e o combobox da configuração de
  execução.
- `press` envia tecla de forma física (CGEventPost), mesmo sem `--headed`. Usado uma vez, para
  fechar um menu aberto por engano.
- Drawer de Workspace tools continuou mostrando "No launcher" depois da instalação.

## Lições

- Antes de declarar um aceite de app real, rodar `agent-desktop status` e `pgrep -x openridr`.
- Não encerrar processo de outra sessão para liberar o caminho. Pedir ao dono.
- Specs que pulam (`skip`) não contam como passe; use `--reporter=list` para identificar.
- Quando uma falha da lista de conhecidas aparecer, comparar com `main` antes de atribuir à
  branch. Foi o que separou a falha de `find-usages` de uma regressão.
- O elemento certo para o clique direito importa: o agrupamento de projetos da solução não é um
  diretório. Confirmar no spec qual nó o teste usa.
- Dados pessoais e caminhos locais saem das evidências com `sed` e uma varredura final com `grep`.

## Próximos passos

1. Completar o aceite restante: 06, 09, 11, tooltips do 12, "Run Unit Test" do 10, "Start
   analysis" do 01 e menus de arquivo, projeto e solução do 05.
2. Investigar Enter no Find, o drawer desatualizado e o duplo Shift pelo driver.
3. Investigar a falha de `find-usages` "Partial coverage" no `main`, separada desta branch.
4. Identificar o teste pulado da rodada completa de Playwright.
