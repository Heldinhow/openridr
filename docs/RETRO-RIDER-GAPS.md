# Retro: gaps OpenRidr × Rider

Data: 04/11/2026. Branch `docs/rider-gaps`. Entregáveis: `RIDER-BEHAVIORS.md`, `GAPS-RIDER-RIDR.md` e este retro. Nenhuma correção de código foi feita.

## Segunda rodada (04/11, ao vivo no Rider)

Com Acessibilidade e Gravação de Tela concedidas:

- **Funcionou:**
  - Foco na janela (`focus-window`) e screenshots (`screenshot --app`, salvos em `.scratch/rider-gaps/`).
  - Snapshots rasos e por região da árvore de acessibilidade, com refs qualificados.
  - Leitura da barra de menus completa (File, Edit, View, Navigate, Code, Build, Run, Tests, Tools, Git, Window, Help). Os itens são legíveis mesmo com os menus fechados. Isso confirmou a existência de funções que o OpenRidr ainda não expõe.
  - Leitura de `git status` no projeto do usuário, só leitura.
- **Não funcionou:**
  - Clique direito em itens da árvore (projeto, arquivo, pasta): erro `ELEMENT_NOT_FOUND` em `AXRole` ou `APP_UNRESPONSIVE`. Nenhum menu de contexto apareceu.
  - Itens da barra de menus não são endereçáveis (`offscreen`, sem `ref_id`). Não há clique semântico nos menus do topo.
  - `list-surfaces` falhou com `AXRole`.
  - Clique semântico em botões da barra lateral: `POLICY_DENIED` (sem caminho semântico; o fallback físico exige `--headed`).
  - Clique físico (`--headed`) no botão Tests: `APP_UNRESPONSIVE`, `delivered_unverified`. Nada abriu.
  - Atalho ⌘⇧A (Find Action) e digitação: o atalho foi `delivered_unverified` e a janela não mudou. O `type` falhou por falta de argumento, então nenhum texto foi digitado.
- **Efeitos colaterais possíveis (não confirmados):** os cliques físicos e o atalho podem ter chegado ao Rider sem efeito visível. Nada foi salvo, e não usei Shift+F10 (o Run do Rider). Confirmar visualmente o estado do Rider ao retomar.
- **Projeto do usuário:**
  - Não executei o projeto de bot (seu `Main` lê credenciais de ambiente e pode operar contra APIs reais). Sem Run, Debug ou teste de execução do app.
  - Fontes intactas: `git status` mostra só arquivos de `bin/` e `obj/` modificados (já rastreados no repositório) e itens não rastreados (`.idea/`, `*.DotSettings.user`, `global.json`), que não foram criados por mim.
  - O Rider está no modo Trial.
- **Processo do Rider:** ativo (estado S, cerca de 5% de CPU). Não parece travado; a falha está na camada de acessibilidade e no foco.
- **Nova tentativa (mesma sessão):** Rider em foco e snapshots respondendo. Clique direito semântico em um item de projeto (`Gabagool.Tests`) voltou com `ELEMENT_NOT_FOUND` em `AXRole` e `delivered_unverified`. O screenshot seguinte mostrou que nenhum menu abriu; só a linha ficou destacada. Não repeti para não arriscar efeitos colaterais.

Com isso, as áreas que dependem de interação (Debug, Tests, Find in Files, Find Usages, aba Build) continuam sem comportamento verificado ao vivo nesta rodada. O que foi visto é a estrutura dos menus e a interface estática.

## O que foi verificado

- **OpenRidr, na superfície web isolada** (`verify-openridr/web.sh`, UI 1520 e host 4417, cópia do OrderDesk):
  - Welcome, confiança, árvore da solução.
  - Menus de contexto de solução, projeto, pasta e arquivo.
  - Tests: descoberta, Run all, resultado por teste, detalhe de falha, Debug selected desabilitado.
  - Search Everywhere com filtro e prévia.
  - Find do editor (painel CodeMirror).
  - Breakpoint, Debug até pausar, locals (expandem), Step Over, Stop.
  - Diálogo de configuração de execução.
  - Find Usages sem análise instalada.
  - Build com sucesso.
  - Settings > Keymap.
- **Keymap base do pacote do Rider**, lido passivamente (atalhos apenas, nada copiado).
- **Descritores de plugin do Rider**: contagem de ações e grupos.
- **Comparativo anterior** (29/10) reconferido em dois pontos. As variáveis expansíveis e o rodapé do diálogo de configuração já estão resolvidos.

## O que NÃO foi verificado

- **Rider, na UI (primeira rodada):** nenhuma, por falta de permissão. Na segunda rodada, a estrutura dos menus e a interface estática foram lidas ao vivo. Comportamento de Debug, testes, Find e Build segue sem verificação (ver seção acima).
- **Janela nativa do OpenRidr (Tauri):** não foi aberta. Os testes foram na superfície web, que usa a mesma UI mas não a janela nativa.
- **Análise (csharp-ls):** "Install analysis" não foi acionado, porque baixa software de rede sem pedido explícito. Por isso Find Usages, Rename e code actions não foram testados de verdade.
- **Build com erro:** não provocado, então os links `arquivo:linha` no Build output continuam sem verificação (G20).
- **Modificadores do keymap base no macOS:** a correspondência Ctrl→⌘ e Alt→⌥ é presumida (G02, G04, G05).

## O que o REA deu de evidência

- **Pouco.** Abri o executável `Rider` (Mach-O arm64) e obtive o inventário do binário.
- `search_strings` falhou: nenhum provedor configurado. Ghidra precisa de `GHIDRA_INSTALL_DIR` e Java; Hopper não está instalado; IDA e Kuna não foram configurados.
- O REA não é o caminho para isto. Os textos de UI e os keymaps estão em jars e recursos, não no launcher. O keymap e os descritores foram lidos por `unzip`, o que é leitura passiva de artefato, não análise REA.
- Para dar valor ao REA, seria preciso configurar um provedor (Ghidra ou Hopper) e abrir o `rider-frontend.jar` ou a biblioteca `intellij.platform.ide.impl.jar` via `inspect_artifact`. Não foi feito.

## Bloqueios e próximos passos

1. **Permissão do macOS (do usuário):** em Ajustes do Sistema > Privacidade e Segurança, liberar Acessibilidade e Gravação de Tela para o app que roda o terminal. Depois, rodar `agent-desktop status` e repetir a observação visual do Rider nos itens de "Verificações pendentes".
2. **Análise do OpenRidr:** decidir se o "Install analysis" entra no teste de Find Usages (G01). Exige aprovação, porque baixa ferramenta.
3. **Keymap:** decidir G02 e G05 e registrar em `docs/decisions.md` antes de mudar atalhos.

## Erros e ajustes deste retro

- Primeiro ⌘F foi enviado sem foco no editor e não abriu o painel. Depois de clicar no editor, abriu. Isso virou G06.
- O cursor do teste de Find Usages ficou na linha 5 em vez da linha 3. O aviso de "análise necessária" aparece independentemente, então a conclusão não muda, mas o teste não foi limpo.
- `docs/RIDR-VS-RIDER.md` ficou desatualizado em dois pontos (variáveis e rodapé). Não foi editado, porque está fora do escopo. Sugestão: atualizar em um commit próprio.
- Os screenshots ficaram em `.scratch/rider-gaps/` e não foram versionados. `.playwright-mcp/` também é ignorado pelo git.
