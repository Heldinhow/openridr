# Glossário

- **Solution Explorer**: árvore da solução aberta (projetos, dependências, arquivos). Mostra o modelo avaliado pelo MSBuild quando existe; senão, um outline de texto.
- **Tool window**: painel acoplado a uma faixa lateral ou inferior (Build, Problems, Debug, Terminal, Tests…). Cada slot mostra uma por vez.
- **Run configuration**: configuração nomeada de execução (projeto, argumentos, ambiente) escolhida na barra superior; serve para Run e Debug.
- **Startup project**: projeto console que Run e Debug executam; implícito quando há um só.
- **Debug preparation**: o que o clique em Debug na barra faz antes de lançar quando algo falta — confirmar a flag experimental, preparar o adaptador, restore, avaliação e build Debug.
- **Recent project**: solução ou projeto aberto antes e listado na tela inicial, com nome, caminho e último acesso. Remover da lista não altera sua Trust decision.
- **Trust decision**: escolha do usuário, uma vez por projeto, entre confiar (*trusted*) e abrir em modo restrito (*restricted*). Persiste entre sessões e pode ser mudada depois.
- **Trust prompt**: a pergunta única que oferece a Trust decision na primeira abertura de um projeto. Não reaparece depois de decidida.
- **Restricted mode**: abertura sem executar código do projeto (sem build, run, debug, análise); só edição de texto.
