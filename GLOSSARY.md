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
- **Conditional breakpoint**: breakpoint que só para quando uma expressão C# do projeto é verdadeira no momento do hit.
- **Hit count**: número de paradas no mesmo breakpoint antes de o debugger parar; conta só os hits aprovados pela condição. _Avoid_: hitCondition (nome do DAP no adaptador).
- **Watch**: expressão C# fixada pelo usuário no painel Debug e reavaliada a cada pausa contra a thread/frame selecionados.
