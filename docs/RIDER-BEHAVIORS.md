# Comportamento do Rider, por área

Data: 04/11/2026. Referência: JetBrains Rider 2026.2.3.1 (build RD-262.10968.170), instalação local no macOS arm64.

Este documento descreve comportamentos com as nossas palavras. Capturas, XML de keymap e strings copiadas ficam em `.scratch/rider-gaps/` (fora do git). Cada item traz a fonte; o que não foi visto nesta rodada está marcado **não verificado**.

## Limites desta rodada

- **Nenhuma observação visual do Rider.** O `agent-desktop status` reportou Acessibilidade e Gravação de Tela negadas para o terminal. Sem isso não há leitura da árvore de UI nem captura de tela do Rider.
- **Fontes usadas:**
  - Keymap padrão do pacote (leitura passiva do arquivo de keymap base, sem copiar o conteúdo).
  - Descritores de plugin (44 arquivos de plugin lidos; apenas contagem e identificadores).
  - Comparativo anterior, `docs/RIDR-VS-RIDER.md`, de 29/10/2026. Os itens marcados "observado em 29/10" vêm dele e não foram reobservados.
- **REA:** abriu o executável do launcher (Mach-O arm64) e não teve provedor de análise disponível (Ghidra sem Java e sem `GHIDRA_INSTALL_DIR`; Hopper ausente; IDA e Kuna não configurados). Nenhuma análise de código ou de strings rodou.
- **Modificadores:** o keymap base usa `Ctrl`/`Alt` genéricos. A correspondência para ⌘/⌥ no macOS é presumida e não foi conferida no app.

## 1. Solution e projeto

- **Rider:** menus de contexto de solução, projeto, pasta e arquivo **não verificados** nesta rodada. Os descritores de plugin não trazem a estrutura completa dos menus; isso exige observar o menu aberto.
- Atalhos do keymap base relacionados (passivo):
  - Expandir tudo e recolher tudo na árvore (sinal de adição e de subtração com Ctrl).
  - Mostrar o caminho do arquivo (Ctrl+Alt+F12).
  - Sincronizar a árvore com o editor (Ctrl+Alt+Y), presumivelmente o "localizar arquivo aberto"; não confirmado.
  - Renomear símbolo (Shift+F6).

## 2. Debug

- **Observado em 29/10:** Debug faz build e para no breakpoint. O painel mostra threads, call frames, variáveis, console de debug e saída do programa. Há barra com Resume, Pause, Step Over, Step Into e Step Out, e campo de avaliação de expressão. Variáveis expansíveis: o comparativo de 29/10 registrou objeto sem expansão; **não reverificado** no Rider.
- Atalhos do keymap base (passivo):
  - Step Over: F8. Step Into: F7. Step Out: Shift+F8.
  - Resume: F9. Stop: Ctrl+F2.
  - Toggle breakpoint: Ctrl+F8. Ver breakpoints: Ctrl+Shift+F8.
  - Run to cursor: Alt+F9. Avaliar expressão: Alt+F8.
  - Debug: Shift+F9.
- Breakpoint em linha sem código (linha em branco): **não verificado**.

## 3. Tests

- **Rider:** descoberta, árvore de resultados, rerun e cobertura **não verificados** nesta rodada.
- Atalhos do keymap base (passivo):
  - Rerun de testes que falharam: Shift+Alt+R.
  - Executar classe: Ctrl+Shift+F10.
- O keymap base atribui Ctrl+U a "ir para o super-método", não a uma sequência de testes. A sequência ⌘U seguida de tecla, usada pelo OpenRidr, **não aparece** no keymap base; a origem no Rider não foi confirmada.
- Descritores do pacote incluem a ação de rerun de testes que falharam.

## 4. Run

- **Observado em 29/10:** o botão de execução faz build e roda. A saída vai para uma janela Run separada. A configuração é derivada do projeto de inicialização. Os campos de edição e os perfis não foram observados nesta rodada.
- Atalhos do keymap base (passivo): Run com Shift+F10; Rerun com Ctrl+F5.

## 5. Console e log

- **Observado em 29/10:** console de debug e aba de saída do depurado.
- **Build output:** cores, busca, wrap, autoscroll e links para `arquivo:linha` **não verificados**.
- **Event log:** não verificado.

## 6. Find

- Atalhos do keymap base (passivo):
  - Find: Ctrl+F. Replace: Ctrl+R.
  - Find in path: Ctrl+Shift+F. Replace in path: Ctrl+Shift+R.
  - Find usages: Alt+F7.
  - Ir para classe: Ctrl+N. Ir para arquivo: Ctrl+Shift+N. Ir para símbolo: Ctrl+Shift+Alt+N.
  - Arquivos recentes: Ctrl+E. Ir para ação: Ctrl+Shift+A.
  - Próxima e anterior ocorrência: F3 e Shift+F3; Ctrl+L também aparece como próxima ocorrência.
- **Search Everywhere:** o keymap base não tem atalho de texto para ele. O gesto de duplo Shift é o padrão conhecido do IDE, **não verificado** nesta rodada.
- Opções de busca (caixa, regex, palavra inteira), prévia e escopo no Rider: **não verificados** nesta rodada.

## 7. Experiência geral

- Atalhos do keymap base:
  - Esconder a janela de ferramenta ativa: Shift+Esc.
  - Janelas de ferramenta: Alt+1 a Alt+8 (no XML, que no macOS corresponde a ⌘ conforme a convenção do keymap; não conferido).
  - Intenções e correções rápidas: Alt+Enter.
- **Observado em 29/10:** tela inicial do Rider não exercitada (a solução foi aberta em nova janela). Nenhum diálogo de confiança apareceu para a pasta de teste. O cabeçalho tem projeto, branch, Build, configuração, Run, Debug, busca e Settings.
- Estados vazios, barra de status, progresso e mensagens de erro: **não verificados**.
- Onboarding: **não verificado**.

## O que falta para fechar as lacunas marcadas

- Conceder Acessibilidade e Gravação de Tela ao app que roda o terminal em Ajustes do Sistema > Privacidade e Segurança. Essa ação é do usuário; a sessão não a faz sozinha.
- Com permissão, observar: menus de contexto de solução, projeto, pasta e arquivo; estado do Debug com variáveis expansíveis; painel de testes; Find in Files e Find Usages; aba Build.
