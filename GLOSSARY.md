# Glossário

- **Solution Explorer**: árvore da solução aberta (projetos, dependências, arquivos). Mostra o modelo avaliado pelo MSBuild quando existe; senão, um outline de texto.
- **Tool window**: painel acoplado a uma faixa lateral ou inferior (Build, Problems, Debug, Terminal, Tests…). Cada slot mostra uma por vez.
- **Run configuration**: configuração nomeada de execução (projeto, argumentos, ambiente) escolhida na barra superior; serve para Run e Debug.
- **Startup project**: projeto console que Run e Debug executam; implícito quando há um só.
- **Build implícito**: o `build.start` que o host inicia sozinho quando Run ou Debug não têm um build atual utilizável (ausente, stale, falho, TargetPath/assembly sumido, fonte salva alterada). A aba do lançamento mostra `building`; só o exit 0 observado libera o processo. Não restaura: `restore_required` continua sendo uma ação explícita do usuário.
- **Debug preparation**: o que o clique em Debug na barra faz antes de lançar quando falta o adaptador — confirmar a flag experimental e planejar/instalar o netcoredbg. Não faz mais restore, avaliação nem build: o host constrói implicitamente quando o build não serve.
- **Recent project**: solução ou projeto aberto antes e listado na tela inicial, com nome, caminho e último acesso. Remover da lista não altera sua Trust decision.
- **Trust decision**: escolha do usuário, uma vez por projeto, entre confiar (*trusted*) e abrir em modo restrito (*restricted*). Persiste entre sessões e pode ser mudada depois.
- **Trust prompt**: a pergunta única que oferece a Trust decision na primeira abertura de um projeto. Não reaparece depois de decidida.
- **Restricted mode**: abertura sem executar código do projeto (sem build, run, debug, análise); só edição de texto.
- **Stop source**: qualquer motivo configurado para o depurador pausar o programa em execução — um breakpoint (quando não mutado), a parada em exceções lançadas, o pause manual ou o run-to-cursor. Sem stop source, a sessão roda até o fim sem parar.
- **No-stop session**: sessão de debug que terminou sem nunca ter pausado. O fim dela precisa ser explicado, porque não há threads, frames nem locals para mostrar.

- **Working-tree diff**: o diff que o painel Git mostra de um arquivo alterado da árvore de trabalho; a base é o index (arquivo sem stage) ou o `HEAD` (arquivo todo em stage). Nunca é um diff de commit.
- **Diff base**: o lado esquerdo do diff — a versão contra a qual o lado atual é comparado. A do working-tree diff é o index ou o `HEAD`, nunca os dois.
- **Diff mode**: a forma de mostrar o working-tree diff: *split* (dois editores lado a lado) ou *unified* (um editor só, com prefixos `-`/`+`). A escolha é do painel, não do arquivo.
- **HEAD commit**: o commit para onde o `HEAD` aponta; é o hash mostrado no diff como contexto da comparação. Não confundir com o blob hash do index, que identifica conteúdo e não revisão.
- **Conditional breakpoint**: breakpoint que só para quando uma expressão C# do projeto é verdadeira no momento do hit.
- **Hit count**: número de paradas no mesmo breakpoint antes de o debugger parar; conta só os hits aprovados pela condição. _Avoid_: hitCondition (nome do DAP no adaptador).
- **Watch**: expressão C# fixada pelo usuário no painel Debug e reavaliada a cada pausa contra a thread/frame selecionados.
- **Local test remote**: a bare repository under the temporary directory plus two throwaway clones — the one the app opens (`user`) and the counterpart (`peer`) — used to exercise real Git operations with no network and no credentials. Never a project of the maintainer and never an external remote.
- **Counterpart (peer)**: the second clone of the local test remote that publishes commits and branches the workspace is expected to receive. Driven outside the app so the effect is real, not simulated.
- **Behind count**: the distance between `HEAD` and the local tracking ref, shown as `↓N`. It is only as fresh as the last fetch or push; an `Update…` (pull) fetches and may incorporate work while the count still reads zero.
- **Divergent branch**: a branch whose local commits and upstream commits both moved, so a pull cannot fast-forward. Git refuses it until a reconciliation is chosen; the app has no such choice today.
