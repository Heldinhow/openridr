# Gaps do OpenRidr em relação ao Rider

Data: 04/11/2026. Branch `docs/rider-gaps`. Base: `docs/RIDER-BEHAVIORS.md` (comportamento do Rider) e observação do OpenRidr na superfície web isolada (`verify-openridr/web.sh`, mesma UI do app Tauri, sem a janela nativa).

Legenda:

- **Prioridade:** P0 bloqueia um fluxo central; P1 é diferença visível que atrapalha a paridade; P2 é refinamento.
- **Esforço:** S (até 1 dia, isolado em UI ou keymap); M (1 a 3 dias, toca host e UI); L (mais de 3 dias ou depende de adaptador novo).
- **"Rider"** traz o que foi observado, o que vem do keymap base ou o que é **não verificado**.

## Gaps por área

### Find

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G01 | Find Usages, Rename e code actions dependem de análise que não está instalada | Análise vem com o IDE (**não verificado** nesta rodada) | Find Usages mostra "Find Usages unavailable: Start analysis before finding usages". A barra de status diz "Analysis: not installed" com botão "Install analysis" | P0 | M | Em projeto confiável com SDK resolvido e sem análise, Find Usages num símbolo oferece instalar e iniciar análise a partir do próprio aviso; após a análise, lista as ocorrências. Spec com csharp-ls real ou `skip` explícito com motivo |
| G02 | Atalho de Find Usages diverge do keymap base | Menu Edit do Rider mostra Find Usages com **⇧F12** (**visto ao vivo**), igual ao perfil rider-macos do OpenRidr; a execução não foi feita. Keymap base: Alt+F7 (**o perfil Rider de macOS do OpenRidr usa Shift+F12**; o perfil IntelliJ usa Alt+F7). Confirmar no app | Dica da barra: "Shift+F12: find usages" | P1 | S | Decisão registrada em `docs/decisions.md` com a fonte. Se mantido, justificativa; se trocado, atalho, dica e Settings mudam juntos |
| G03 | Search Everywhere sem atalho em nenhum perfil | Rider mostra a dica de Search Everywhere com duplo Shift ou ⌘T (**visto ao vivo**, tela inicial do editor). OpenRidr não tem atalho para isso | Dica vazia na tela inicial do editor ("Search Everywhere" sem tecla); Settings: ⇧⌘A é "Find Action" | P1 | S | Duplo Shift abre Search Everywhere, ou atalho documentado aparece na Settings e na dica |
| G04 | Janelas de ferramenta sem atalho | Keymap base: Alt+1 a Alt+8 (**confirmar no macOS**). Ao vivo, a ação "Select Opened File" da Solution aparece rotulada com ⌥F1 | Settings lista "No shortcut" para Solution Explorer, Commit, Build, Run, Problems, Debug, Terminal, Tests, Git e NuGet | P1 | S | Atalho alterna a janela de ferramenta (abre, foca, fecha); aparece na Settings; teste de keymap |
| G05 | Atalho de pesquisa por símbolo diverge do keymap base | Keymap base: Ctrl+Shift+Alt+N (**confirmar**). OpenRidr (rider-macos): ⌥⌘⇧T | Settings mostra ⌥⌘⇧T | P2 | S | Decisão registrada, como G02 |
| G06 | ⌘F não abre a busca do editor quando o foco está na árvore ou no Search Everywhere | **Não verificado** | Após abrir `Order.cs` pela busca, ⌘F não mostrou painel; após clicar no editor, abriu | P2 | S | Ao abrir arquivo por qualquer caminho, o foco vai para o editor, ou ⌘F age no editor ativo |
| G07 | Painel de busca do editor é o padrão do CodeMirror | Contador de ocorrências, opções de caixa, regex e palavra inteira, prévia e busca na seleção: **não verificados** | Próximo, anterior, todos, caixa, regex, palavra, substituir e substituir tudo. **Sem contador** de ocorrências | P2 | M | Painel mostra "n de m"; opção "na seleção" existe; teste de regressão |

### Tests

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G08 | Descoberta de testes é manual | Descoberta automática: **não verificada** nesta rodada | Painel mostra "No discovery has run" e exige "Discover tests". Após o clique, 4 testes agrupados por classe, status "not run" | P1 | M | Solução com SDK resolvido e restaurada lista os testes sem clique; falha de descoberta mostra a causa no painel |
| G09 | Caminho e linha da stack trace não são links | Link arquivo:linha na stack trace abre o arquivo no editor já na linha (**visto ao vivo**) | Saída do teste mostra `OrderTests.cs:line 29` como texto; no DOM não há `a`, `button` nem `role=link` com esse texto | P1 | S | Clicar em `arquivo:linha` na saída de teste abre o arquivo na linha |
| G10 | Debug de teste indisponível | Depurar teste unitário para no breakpoint e mostra threads, pilha, variáveis e avaliação (**visto ao vivo**) | "Debug selected" desabilitado com motivo: precisa de attach ao test host, sem suporte | P1 | L | Decisão de escopo registrada; enquanto isso, o motivo continua visível. Quando houver suporte, teste de regressão com breakpoint |
| G11 | Atalhos de teste no perfil Rider: só ⌘U + tecla, sem remapeamento | Ctrl+U no keymap base é "super-método", não teste (**confirmar a origem do chord no Rider**) | Perfil rider-macos: sem atalho direto. Chords ⌘U+R/D/L/F existem e "não são remapeáveis ainda" (`keymap.ts`) | P2 | M | Chords aparecem em Settings e podem ser remapeados, ou decisão registrada manteve o modelo atual |
| G12 | Sem navegação entre falhas | Menu Tests tem "Rerun Failed Tests" e "Run Unit Tests Until Fail" (**visto ao vivo**; comportamento não exercitado) | Status "failed" com cor e stack trace por teste (botão Details). Entre os controles do painel não há "próxima falha" | P2 | S | Atalho ou botão leva à próxima falha |

### Solution e projeto

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G13 | Menu da pasta da solução vazio | Menu de pasta com itens de Add, build dos selecionados, ações avançadas e Open In (**visto ao vivo**) | Clique direito em pasta não abre menu (nenhum `role=menu` no DOM) | P1 | S | Pasta abre menu com pelo menos Novo arquivo, Copiar caminho e Localizar no Finder |
| G14 | Sem "Mostrar no Finder" nem "Abrir com" | Open In > Finder existe no Rider (**visto ao vivo**) | Arquivo: Abrir, Copiar caminho, Find in Files, Renomear, Excluir. Projeto: NuGet, Recarregar, Build, Run Unit Test, Editar, Novo arquivo, Copiar caminho, Propriedades | P1 | S | Arquivo, pasta, projeto e solução têm "Mostrar no Finder" que seleciona o item no Finder. Antes de implementar, checar se o host ou o Tauri já expõem essa capacidade (não verificado) |
| G15 | Menu da solução restrito | Build, Rebuild e Clean da solução, build do projeto atual e do de inicialização, e troca de configuração existem no menu Build (**visto ao vivo**; comportamento não exercitado) | Solução: Recarregar, Copiar caminho, Find in Files. Sem build, restore nem adicionar projeto | P1 | M | Lista de itens definida em spec e coberta por teste de menu |
| G16 | Sem mover, adicionar item existente nem excluir pasta | Arquivo tem Excluir e Renomear no submenu Editar; Mover não aparece no menu do arquivo (**visto ao vivo**) | Só Renomear e Excluir em arquivo | P2 | M | Decisão de escopo por item; cada um com teste de E2E e fixture |
| G17 | "Run Unit Test" em projeto de biblioteca sem teste | **Não verificado** | Aparece no menu de OrderDesk.Core (biblioteca), sem indicação de motivo | P2 | S | Item oculto ou desabilitado com motivo quando o projeto não tem testes |

### Run e configurações

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G18 | Configuração derivada é só leitura; editar exige Duplicar | Menu Run tem "Edit Configurations…" (**visto ao vivo**); edição direta não exercitada | Diálogo: "This default configuration is derived ... read only. Duplicate it to edit a named copy" | P2 | M | Editar argumentos e variáveis sem duplicar, ou decisão registrada mantendo o modelo |

### Debug

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G19 | Breakpoint aceito em linha em branco | Menu Run tem "Toggle Breakpoint", "View Breakpoints and Exceptions…" e "Remove All Line Breakpoints" (**visto ao vivo**). Breakpoint de linha de código criado pelo menu (**visto ao vivo**). Linha em branco: **não verificado** | Atalho de breakpoint na linha 5 (em branco) criou marca vermelha ativa | P2 | S | Linha sem código não recebe breakpoint, ou move para a próxima linha executável (comportamento a confirmar no Rider) |

### Console e log

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G20 | Links `arquivo:linha` no Build output | **Não verificado** | Build com sucesso mostra "0 build problems" e caminhos de DLL, sem links `.cs`. **Não testado com erro de build** | P1 | S | Build com erro proposital mostra link clicável para o arquivo e a linha (teste E2E) |

### Experiência geral

| # | Gap | Rider | OpenRidr hoje (observado) | P | E | Critério de aceite |
|---|---|---|---|---|---|---|
| G21 | Rótulo "Keymap: IntelliJ / Rider" na barra de status é ambíguo | Não se aplica | Texto literal na barra de status | P2 | S | Rótulo mostra o nome exato do perfil ativo |
| G22 | Três janelas de ferramenta desabilitadas à direita | Não se aplica | Monitoramento (M), AI Assistant (A) e Database (D) aparecem desabilitadas | P2 | S | Ocultar até existirem, ou tooltip explicando |
| G23 | Ícones sem rótulo visível na faixa da Solution | **Não verificado** | Mostrar todos os arquivos, buscar, selecionar arquivo aberto, expandir, recolher, opções, ocultar | P2 | S | Tooltip e nome acessível em cada ícone |

## Já resolvido desde o comparativo de 29/10

Verificado nesta rodada, no mesmo workspace isolado:

- Variáveis expansíveis no Debug. Em 29/10 o objeto aparecia como "nested; not expanded". Hoje `order` expande para `Id` e `Total`.
- Diálogo de configuração tem OK, Run, Apply e Cancel. O comparativo de 29/10 apontava um rodapé sem Run/Apply/OK.

## Os 10 gaps de maior valor

1. **G01**: Find Usages, Rename e code actions só funcionam depois de instalar a análise. (P0, M)
2. **G04**: janelas de ferramenta sem atalho. (P1, S)
3. **G03**: Search Everywhere sem atalho. (P1, S)
4. **G08**: descoberta de testes manual. (P1, M)
5. **G09**: stack trace de teste sem link para o arquivo. (P1, S)
6. **G14**: sem "Mostrar no Finder" em arquivo, pasta, projeto e solução. (P1, S)
7. **G13**: menu de pasta vazio. (P1, S)
8. **G15**: menu da solução restrito. (P1, M)
9. **G02**: atalho de Find Usages diverge do keymap base; decisão necessária. (P1, S)
10. **G20**: links `arquivo:linha` no Build output, a verificar com erro de build. (P1, S)

Fora da lista e com esforço L: G10 (debug de teste, que exige attach ao test host).

## Verificações pendentes

Terceira rodada (04/11, fixture OrderDesk, cliques por coordenada e menus): confirmados ao vivo os menus de contexto, a stack trace clicável, Debug de teste com breakpoint, Run, Search Everywhere e Find in Files. Continuam sem prova: execução de Find Usages (só o menu foi visto), rerun de testes que falharam (sem mudança visível), breakpoint em linha em branco, links `arquivo:linha` no Build output (sem erro de build provocado), e a edição de configuração de execução no Rider (o diálogo Edit Configurations não foi aberto nesta rodada).

Segunda rodada (04/11, com Acessibilidade liberada): o menu do Rider foi lido ao vivo e confirmou a existência de várias funções (marcadas acima). O Rider não reagiu a cliques de contexto nem a atalhos, então o comportamento continua sem verificação:

- Menus de contexto de solução, projeto, pasta e arquivo (G13, G14, G15, G16, G17).
- Painel de testes, descoberta e execução (G08, G09, G10, G11, G12).
- Find in Files, Find Usages e Search Everywhere (G01, G02, G03, G07).
- Aba Build e links de erro (G20).
- Breakpoint em linha em branco (G19).
- Confirmar o mapeamento de modificadores do keymap base para ⌘/⌥ no macOS (G02, G04, G05).

Registro para a comparação com o OpenRidr: o Search Everywhere do OpenRidr tem abas All, Classes, Files, Symbols, Actions e Text, enquanto o menu Navigate do Rider lista classe, arquivo, símbolo e texto. A correspondência de categorias é próxima; o comportamento de cada aba não foi comparado.
