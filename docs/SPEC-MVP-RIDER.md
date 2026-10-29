# OpenRidr — especificação do MVP "Rider-like" (fatias F1–F9)

Status: ready-for-agent
Category: enhancement
Data: 28/10/2026
Fase: especificação. Este arquivo não autoriza criar tickets, implementar, instalar ferramentas nem distribuir; o próximo passo é `/to-tickets`.

Fontes: observação clean-room do Rider 2026.2.3 (`SPEC-MVP-OPENRIDR.md`), handoff do MVP (`HANDOFF-mvp-rider.md`), grill aprovado (D1–D16 em `GRILL-MVP.md`), [SPEC do P0](SPEC.md), [decisões](decisions.md) e o código atual (tickets 01–18 implementados e verificados no Mac).

Este recorte **não refaz** o ciclo já provado do P0 — confiança, avaliação MSBuild, restore explícito, csharp-ls, build/run, debug DAP, PTY e VSTest continuam com os mesmos contratos e gates. Ele troca a casca fixa por uma casca de IDE e acrescenta os fluxos de uso diário que faltam.

## Problem Statement

O OpenRidr já fecha o ciclo abrir solução → editar → compilar → rodar → depurar → testar, mas a casca atual é uma única view de 3.370 linhas com cinco painéis fixos numa faixa inferior. Não existe busca global, configuração de execução salva, Git, Settings, NuGet nem keymap configurável; code actions e rename existem no host, sem a superfície que o usuário espera.

Para quem vem de uma IDE JetBrains/Rider, isso significa decorar onde cada coisa mora e perder a memória muscular (⌘T, ⌥↩, ⌃F5) no momento em que precisa navegar, rodar ou commitar. Cada painel novo empilhado nessa view única também aumenta o risco de regressão nos 1.326 cliques de teste E2E existentes.

O usuário precisa de uma casca previsível — barra, faixas, slots, status, paleta — que exponha o que o host já sabe fazer, sem inventar resultados quando falta trust, análise, build ou repositório.

## Solution

Uma casca de IDE em uma única barra de 44 px (projeto, branch, run config, Run/Debug, busca, Settings), faixas de ícones com slots esquerdo e inferior, status bar com breadcrumb/posição/encoding/indentação, e um registro central de ações que alimenta menus, atalhos e a paleta.

Sobre essa casca: Search Everywhere com seis abas e prévia; run configurations persistidas por solução, com diálogo e seletor; tool window Tests com árvore, filtro, run/debug e rerun failed; Git básico (branch, commit por seleção, log, pull/push confirmados); Settings com busca, keymap e editor; NuGet instalado/desatualizado com atualização confirmada; e, por último, gutter de Run/Test, Alt+Enter com prévia, menu de contexto e rename inline.

Tudo em inglês na UI, documentação em PT-BR, sem cópia de ícones, capturas ou binários do Rider, e sem dependência nova.

## User Stories

### F1 — Casca, tool windows e status

1. Como desenvolvedor C#, quero abrir Build, Problems, Debug, Terminal e Tests pelos ícones da faixa lateral, para não caçar a aba certa.
2. Como desenvolvedor C#, quero que cada tool window ocupe um slot e alterne com a do mesmo slot, para não empilhar painéis e perder o editor.
3. Como desenvolvedor C#, quero redimensionar slots por ponteiro e por teclado, para ajustar a área de código ao que estou fazendo.
4. Como desenvolvedor C#, quero que tamanhos e tool window ativa sobrevivam ao reload, para manter meu arranjo de trabalho.
5. Como desenvolvedor C#, quero esconder a tool window ativa com ⇧Esc, para voltar ao editor sem clicar.
6. Como desenvolvedor C#, quero ver projeto, branch, configuração de execução, Run/Debug, busca e Settings numa barra só, para saber onde estou e o que vai rodar.
7. Como desenvolvedor C#, quero que branch e run config desabilitados expliquem o motivo, para não confundir ausência de recurso com bug.
8. Como desenvolvedor C#, quero status bar com breadcrumb, linha:coluna, encoding e indentação, para me orientar sem abrir menu.
9. Como desenvolvedor C#, quero que alternar de tool window preserve o conteúdo já carregado (log de build, shell, resultados), para não perder contexto.
10. Como desenvolvedor C#, quero navegar faixas e slots só com teclado e foco visível, para trabalhar sem mouse.
11. Como quem mantém o app, quero que a migração da casca mantenha todos os `data-testid` existentes, para que a suíte E2E continue provando o P0.

### F2 — Ações e keymap

12. Como desenvolvedor C#, quero executar qualquer ação pelo nome (⇧⌘A), para descobrir o que o app faz sem decorar menus.
13. Como desenvolvedor C#, quero que menus, atalhos e a paleta venham do mesmo registro, para que nunca divirjam.
14. Como desenvolvedor C#, quero o perfil "Rider (macOS)" por padrão, para manter a memória muscular de quem usa Rider.
15. Como desenvolvedor vindo do IntelliJ, quero um perfil "IntelliJ" opcional, para não reaprender atalhos.
16. Como desenvolvedor C#, quero remapear uma ação, para resolver conflito com o sistema operacional.
17. Como desenvolvedor C#, quero ser avisado quando dois atalhos colidem, para corrigir sem quebrar o app.
18. Como desenvolvedor C#, quero que ações indisponíveis digam o motivo (trust, análise, build), para saber o que falta.
19. Como desenvolvedor C#, quero que ⌘/ (comentar), ⌘F (busca no arquivo) e as setas continuem funcionando no editor, para que a captura global não quebre a edição.

### F3 — Busca global

20. Como desenvolvedor C#, quero abrir a busca com ⌘T ou Shift Shift e encontrar a última aba e o último texto, para repetir a busca anterior.
21. Como desenvolvedor C#, quero as abas All, Classes, Files, Symbols, Actions e Text, para escolher o tipo de resultado.
22. Como desenvolvedor C#, quero abrir um arquivo pelo nome com Enter, para navegar rápido.
23. Como desenvolvedor C#, quero achar um tipo ou membro pelo nome, com contêiner e caminho, para chegar ao símbolo certo.
24. Como desenvolvedor C#, quero buscar texto no workspace com trecho e caminho, para localizar strings e mensagens.
25. Como desenvolvedor C#, quero executar uma ação direto da paleta, para agir sem procurar o menu.
26. Como desenvolvedor C#, quero uma prévia do código com o trecho destacado, para confirmar antes de abrir.
27. Como desenvolvedor C#, quero que a busca diga "Analysis not ready" quando não há trust/análise, em vez de fingir zero resultados.
28. Como desenvolvedor C#, quero saber quando a lista foi truncada, para refinar a busca.
29. Como desenvolvedor C#, quero que Esc feche a busca e devolva o foco ao editor, para continuar digitando.

### F4 — Run configurations

30. Como desenvolvedor C#, quero criar uma configuração com nome, projeto, Configuration, TFM, argumentos, diretório, variáveis e arquivos `.env`, para não redigitar tudo a cada execução.
31. Como desenvolvedor C#, quero escolher a configuração ativa no topo, para saber exatamente o que Run e Debug vão executar.
32. Como desenvolvedor C#, quero rodar e depurar pelo widget com os mesmos gates de restore/build/plataforma, para não ganhar um caminho que burla segurança.
33. Como desenvolvedor C#, quero que mudar Configuration/TFM na barra atualize a configuração ativa, para não haver duas verdades.
34. Como desenvolvedor C#, quero que as configurações fiquem salvas por solução, para reabrir o projeto e continuar de onde parei.
35. Como desenvolvedor C#, quero uma configuração padrão derivada do projeto de startup, pronta para rodar e somente leitura, para começar sem configurar nada.
36. Como desenvolvedor C#, quero que uma configuração inválida explique o campo e a ação antes de executar, para não receber só um exit code.
37. Como desenvolvedor C#, quero que trocar de configuração nunca reuse TargetPath de outra, para não rodar o binário errado.
38. Como desenvolvedor C#, quero ser avisado sobre buffers não salvos antes de build/run, para decidir entre salvar e rodar o build salvo.

### F5 — Tests

39. Como desenvolvedor C#, quero a tool window Tests com árvore por projeto, namespace, classe e método, para achar o teste.
40. Como desenvolvedor C#, quero filtrar a árvore por texto, para chegar a um teste em soluções grandes.
41. Como desenvolvedor C#, quero rodar todos, rodar a seleção e depurar a seleção, para usar o runner que já existe.
42. Como desenvolvedor C#, quero repetir só os testes que falharam, para iterar rápido.
43. Como desenvolvedor C#, quero status por teste e detalhes de falha com log, para entender o que aconteceu.
44. Como desenvolvedor C#, quero um estado vazio honesto quando não há testes descobertos, para não achar que a suíte passou.
45. Como desenvolvedor C#, quero o quick list de testes (⌥⇧U), para agir sem abrir a tool window.
46. Como desenvolvedor C#, quero que descoberta/resultado stale ou parcial fique visível e não selecionável, para não confiar em dado velho.
47. Como desenvolvedor C#, quero que o agrupamento derivado do FQN seja declarado na UI, para não confundir heurística com estrutura real.

### F6 — Git

48. Como desenvolvedor C#, quero ver a branch atual e trocar de branch pelo widget, para mudar de contexto sem sair do app.
49. Como desenvolvedor C#, quero criar uma branch a partir da atual, para começar um trabalho novo.
50. Como desenvolvedor C#, quero ver o log da branch com mensagem, autor e data, para entender o histórico.
51. Como desenvolvedor C#, quero ver o diff de um arquivo alterado, para revisar antes de commitar.
52. Como desenvolvedor C#, quero marcar arquivos, escrever a mensagem e commitar só os marcados, para não commitar o que não revisei.
53. Como desenvolvedor C#, quero fazer amend do último commit, para corrigir mensagem ou conteúdo.
54. Como desenvolvedor C#, quero que o push peça confirmação com remote, branch e commits à frente, para não publicar por acidente.
55. Como desenvolvedor C#, quero que checkout e pull recusem árvore suja com explicação, para não perder alterações.
56. Como desenvolvedor C#, quero que as tool windows de Git expliquem quando não há repositório ou trust, para não parecer falha.
57. Como desenvolvedor C#, quero que minhas credenciais continuem no credential helper do git, para o app não guardar token.

### F7 — Código

58. Como desenvolvedor C#, quero ícones de Run e de teste no gutter, para executar a partir da linha.
59. Como desenvolvedor C#, quero ⌥↩ com quick fixes e intenções do LSP e prévia do diff, para corrigir sem sair do teclado.
60. Como desenvolvedor C#, quero um menu de contexto no editor com Context Actions, Find Usages, Go To, Rename, Run/Debug Test e Git, para achar as ações no lugar esperado.
61. Como desenvolvedor C#, quero renomear em linha, com prévia e revalidação, para renomear sem modal.
62. Como desenvolvedor C#, quero Find Usages (⇧F12) com a lista de ocorrências, para avaliar o impacto antes de mudar.
63. Como desenvolvedor C#, quero que o menu não ofereça Refactor This quando ele não existe, para não clicar em nada morto.

### F8 — Settings

64. Como desenvolvedor C#, quero Settings com busca e árvore, para achar a opção pelo nome.
65. Como desenvolvedor C#, quero trocar o perfil de keymap e remapear ações, para ajustar o app ao meu teclado.
66. Como desenvolvedor C#, quero fonte, tamanho e indentação do editor aplicados ao vivo, para ler e formatar como prefiro.
67. Como desenvolvedor C#, quero ver os SDKs detectados e pedir nova detecção, para diagnosticar ambiente.
68. Como desenvolvedor C#, quero que o tema escuro esteja explícito e o claro ausente com motivo, para não procurar o que não existe.
69. Como desenvolvedor C#, quero que as settings valham para o app inteiro e persistam, para não reconfigurar a cada solução.

### F9 — NuGet

70. Como desenvolvedor C#, quero ver os pacotes instalados por projeto, para saber o que a solução usa.
71. Como desenvolvedor C#, quero ver os desatualizados com versão atual e nova, para decidir a atualização.
72. Como desenvolvedor C#, quero atualizar um pacote com plano de comando e confirmação, para saber o que será executado.
73. Como desenvolvedor C#, quero fazer o restore explicitamente depois do update, para não disparar restore implícito.
74. Como desenvolvedor C#, quero que soluções com CPM recusem update com mensagem, para o app não editar props por adivinhação.
75. Como desenvolvedor C#, quero multi-TFM mostrado por framework, para não ler um resultado misturado.

### Transversal

76. Como quem avalia release, quero que Windows permaneça "não executado" até haver prova real, para não declarar suporte sem evidência.
77. Como quem mantém o app, quero que cada fatia termine com a suíte E2E inteira verde e um commit local, para manter a base reversível.

## Implementation Decisions

### Ordem e dependências

| Fatia | Entrega | Depende de |
|---|---|---|
| F1 | Registro de tool windows, faixas, slots, barra única, status bar; migração dos 5 painéis | nenhuma |
| F2 | Registro de ações, atalhos, perfis, Find Action | F1 |
| F3 | Search Everywhere com 6 abas + `workspace/symbol` e busca textual no host | F2 |
| F4 | Run configurations (modelo, storage, diálogo, widget) ligadas a build/run/debug | F1 |
| F5 | Tool window Tests (árvore, filtro, ações, quick list) | F1 |
| F6 | Git (status, branch, commit, diff, log, pull/push) | F1 |
| F8 | Settings (busca, keymap, editor, build, VCS) | F2 |
| F9 | NuGet (instalados, outdated, update) | F1 |
| F7 | Gutter Run/Test, ⌥↩, menu de contexto, rename inline, Find Usages | F2, F5, F6 |

Ordem aprovada: **F1 → F2 → F3 → F4 → F5 → F6 → F8 → F9 → F7**. F1 e F2 são fundação; F3 consome o registro de ações; F4/F5/F6 dependem da barra; F7 fecha o ciclo e é a fatia mais cara em UI.

### Casca e layout

- Uma única barra de 44 px, reaproveitando a toolbar existente: à esquerda o widget de projeto (nome + popup de soluções recentes) e o widget de branch; no centro Configuration/TFM/startup compactos; à direita o widget de run config com Run/Debug, o botão de busca e Settings.
- Na F1, o widget de branch fica desabilitado com "Git not connected" e o de run config com "Run configurations arrive in F4"; ambos ganham função nas fatias donas. Motivo textual sempre visível em `title` e texto.
- Slots: esquerdo e inferior. O direito fica fora do MVP. Cada slot tem uma tool window ativa; clicar no ícone ativo oculta, clicar em outro alterna, ⇧Esc oculta a ativa. Sem split, detach, reordenar ou flutuar.
- Registro de tool window: id estável, título, slot, ordem na faixa, ícone próprio (SVG), componente e condição de disponibilidade. A faixa e os slots renderizam a partir do registro; nenhum painel é montado fora dele.
- Migração da F1: Build, Problems, Debug, Terminal e Tests passam a ser tool windows do registro, preservando o cabeçalho com `role="tab"` e os `data-testid` atuais. O estado dos painéis continua na view que hoje o detém; a F1 extrai componentes por responsabilidade, **sem** introduzir state manager.
- Layout persistido em `localStorage`: slot sizes, tool window ativa por slot e visibilidade. Chaves namespaced por app; valores fora de faixa são corrigidos ao carregar (sem quebrar o layout).
- Status bar mantém os itens atuais (workspace, modo, SDK, avaliação, buffers sujos, keymap) e acrescenta breadcrumb da solução/projeto/arquivo, `Ln/Col`, encoding e indentação. Encoding é **UTF-8 fixo** (o app lê/grava UTF-8; não há detecção); indentação vem das settings do editor, não de análise do arquivo.
- Acessibilidade: cada ícone de faixa é botão com nome acessível, `aria-pressed` quando ativo e foco visível; resize por setas quando o divisor tem foco; a ordem de tabulação segue barra → faixa → editor → slot.

### Ações e keymap

- Registro central de ações: id namespaced (`toolWindow.build`, `search.everywhere`, `run.start`, `git.commit`), título, grupo, atalho padrão por perfil, disponibilidade com motivo, executar. Menus, botões de barra, faixas e a paleta leem do registro.
- Atalhos descritos por tecla + modificadores por plataforma, não por string: `⌘T` no Mac vira `Ctrl+T` no Windows; `⌥` vira `Alt`. Um atalho pode ter uma tecla secundária.
- Precedência: remap do usuário > perfil escolhido > padrão da ação. O handler global roda em bubble, ignora `defaultPrevented` e ignora `input/textarea/contenteditable`, exceto ações marcadas como de editor (ex.: comentar linha, que o CodeMirror resolve).
- Colisão entre duas ações no mesmo atalho gera warning no registro e mantém ambas listáveis/remapeáveis; a primeira registrada vence em execução.
- Perfis: "Rider (macOS)" (padrão) e "IntelliJ". Remaps persistem em `settings.json`.

Atalhos mínimos do MVP (Mac / Windows):

| Ação | Rider (macOS) | IntelliJ |
|---|---|---|
| Search Everywhere | ⌘T e Shift Shift | Shift Shift |
| Ir para arquivo | ⇧⌘T / Ctrl+Shift+T | ⌘O / Ctrl+O |
| Ir para símbolo | ⌥⇧⌘T / Ctrl+Alt+Shift+T | ⌥⌘O / Ctrl+Alt+O |
| Buscar texto | ⌥⇧⌘E / Ctrl+Alt+Shift+E | ⇧⌘F / Ctrl+Shift+F |
| Find Action | ⇧⌘A / Ctrl+Shift+A | ⇧⌘A / Ctrl+Shift+A |
| Ir para linha | ⌘G / Ctrl+G | ⌘L / Ctrl+L |
| Declaração/definição | F12 (⌘B secundário) | ⌘B / Ctrl+B |
| Find Usages | ⇧F12 / Shift+F12 | ⌥F7 / Alt+F7 |
| Context Actions | ⌥↩ / Alt+Enter | ⌥↩ / Alt+Enter |
| Rename | ⌘R R (⇧F6 secundário) | ⇧F6 / Shift+F6 |
| Run 'config' | ⌃F5 / Shift+F10 | ⌃F5 / Shift+F10 |
| Debug 'config' | ⌥F5 / Shift+F9 | ⌥F5 / Shift+F9 |
| Stop | ⇧F5 / Shift+F5 | ⇧F5 / Shift+F5 |
| Testes (acorde) | ⌘U R/D/L/F | ⌘U R/D/L/F |
| Quick list de testes | ⌥⇧U / Alt+Shift+U | ⌥⇧U / Alt+Shift+U |
| Commit | ⌥⇧⌘K / Ctrl+Alt+K | ⌘K / Ctrl+K |
| Push | ⇧⌘K / Ctrl+Shift+K | ⇧⌘K / Ctrl+Shift+K |
| Update (pull) | ⌥⌘W / Ctrl+Alt+W | ⌥⌘W / Ctrl+Alt+W |
| Nova branch | ⌥⌘N / Ctrl+Alt+N | ⌥⌘N / Ctrl+Alt+N |
| Esconder tool window ativa | ⇧Esc | ⇧Esc |

`⌃F5`/`⌥F5` no Mac usam Control/Option (não Command), como no keymap Rider observado. No Windows, o perfil Rider usa as teclas do Visual Studio; o perfil IntelliJ mantém `Ctrl+Shift+F10`/`Shift+F9` para run/debug, conforme o keymap IntelliJ.

### Busca global

- Componente único, nascido como Find Action na F2 e evoluído para Search Everywhere na F3. Abas: All, Classes, Files, Symbols, Actions, Text. Tab (e ⇧Tab) alterna; setas navegam; Enter abre; Esc fecha e devolve foco ao editor; o último texto e a última aba persistem.
- Shift Shift: duplo toque de Shift em até 400 ms, ignorado quando o foco está em campo de texto.
- Fontes: **Files** = walk do root do workspace, ignorando `bin/`, `obj/`, `.git/`, `node_modules/` (cap 5.000 arquivos); **Classes/Symbols** = `workspace/symbol` do csharp-ls via novo método de host, separando tipos de membros pelo `kind` (cap 500); **Actions** = registro; **Text** = busca textual no host, sem `ripgrep` (cap 200 arquivos e 2.000 ocorrências, arquivos acima de 1 MiB ignorados).
- Estados honestos: sem trust/análise pronta, Classes/Symbols mostram "Analysis not ready" com a ação que falta; sem resultados reais, mostram "No results" (nunca uma lista vazia indistinguível de falha). Truncamento é declarado com o número devolvido.
- A prévia mostra caminho e trecho com a ocorrência destacada. Abrir em split fica fora (S6).
- A busca não executa código do repositório; Text e Files leem arquivos, Symbols depende da sessão LSP já autorizada.

### Run configurations

- Modelo por solução: id, nome, projeto, Configuration, TFM, argumentos, diretório de trabalho, variáveis de ambiente e arquivos `.env`. A **configuração ativa é a fonte da verdade** desses campos; a barra escreve nela e dispara `evaluation.run`.
- Persistência: um arquivo por solução no app home, nomeado por `fnv1a64` do caminho canônico da solução, com a lista de configs e o id ativo. Nada dentro do repo.
- Configuração padrão `Run <StartupProject>`, derivada da avaliação (projeto console, Configuration e TFM correntes), somente leitura; editar abre o diálogo em modo "duplicar", criando uma config nomeada.
- Diálogo modal com validação por campo e rodapé Run/Cancel/Apply/OK. Sem "múltiplas instâncias", "anexar a filhos", "abrir navegador" nem templates — não se aplicam ao recorte.
- Run/Debug chamam os fluxos existentes: `build.start` e, no sucesso, `run.start`/`debug.start`, com os gates de restore, plataforma e revisão já implementados. Nenhum pipeline novo e nenhum restore implícito.
- Trocar Configuration/TFM/projeto na barra atualiza a config ativa e dispara `evaluation.run`; a UI nunca reutiliza TargetPath de outra configuração.
- Buffers não salvos: antes de build/run, a UI pergunta entre "Save all and build", "Build saved content (N unsaved buffers excluded)" e "Cancel"; nunca salva sozinha.
- Arquivos `.env`: uma linha `KEY=VALUE` por variável, `#` inicia comentário, sem expansão de variáveis e sem shell. Linha inválida é reportada no diálogo, não ignorada em silêncio.

### Tests

- Reusa descoberta e execução VSTest existentes. A tool window é um tool window do registro, não um painel novo.
- Árvore: projeto > prefixo do FQN > classe > método, derivada do nome completamente qualificado. A UI declara "Grouping derived from test names" e mostra o FQN completo em tooltip. Sem parser de fonte e sem posição inventada.
- Ações: Run all, Run selected, Debug selected, Rerun failed, Stop, Refresh (nova descoberta). Rerun failed usa os ids `failed`/`notExecuted` do último run completo e fica desabilitado quando o resultado está stale/sealed.
- Quick list ⌥⇧U: popup numerado com Run all, Run selected, Debug, Rerun failed e Stop. "Teste atual no cursor" fica para a F7.
- Estados da árvore: descoberta em andamento, vazio, adapter ausente, MTP não suportado, falha, stale/sealed e parcial — todos com texto próprio, sem transformar ausência em sucesso.
- Filtro por texto preserva os grupos dos itens que casam e não altera a seleção.

### Git

- Executa o `git` do PATH no host; sem libgit2, sem instalar git, sem repositório embutido.
- Operações Git que consultam o repositório ou executam comandos exigem trust. Em Restricted mode, as tool windows dizem "Trust the workspace to use Git"; sem repositório, "Not a git repository". Nenhum comando de escrita roda antes de confirmação. Settings pode resolver apenas o caminho do executável no PATH sem trust; consulta de versão/repositório continua gated.
- Leitura: status (`--porcelain=v1 -z`), diff por arquivo, branches locais/remotas, log da branch atual (`--max-count=200`, com parents).
- Commit: seleção por checkbox define os caminhos; o commit é `git commit --only -- <caminhos>` com a mensagem, e `--amend` quando marcado. O index do usuário não é alterado; desmarcar não faz `restore --staged`. Hooks do repositório rodam (sem `--no-verify`).
- Branch: criar a partir da atual; checkout de branch local ou remota. Checkout e pull recusam árvore suja, merge/rebase/cherry-pick em andamento, com mensagem e sem auto-stash.
- Push: diálogo mostra remote, branch local, upstream, commits à frente e arquivos; confirmação explícita. Sem `--force`, sem publicar tags ou branches não pedidas.
- Log: lista de commits com mensagem, autor, data e parents; selecionar mostra os arquivos alterados (sem grafo, sem paginação no MVP).
- Atualização da UI: ao abrir/focar a tool window, depois de cada operação e por botão de refresh; sem polling.
- Credenciais: o app herda o credential helper do usuário e nunca guarda token, senha ou chave.

### Settings

- Diálogo modal com busca no topo e árvore lateral; Cancel/Save explícitos. Escopo global (app inteiro), sem override por projeto.
- Páginas do MVP: Appearance (tema escuro fixo com o claro desabilitado e motivo; zoom/densidade), Keymap (perfil + remap por ação, com reset), Editor (família, tamanho, indentação e tabs), Build (SDKs detectados, host de análise e Redetect), Version Control (caminho do git, read-only).
- Alterações aplicam ao vivo ao confirmar; `settings.json` no app home. Valores inválidos são rejeitados com mensagem e não gravam.
- Settings não muda `global.json`, não instala SDK e não altera a política de trust.

### NuGet

- Por projeto da solução avaliada: instalados (`dotnet list <csproj> package --format json`) e desatualizados (`--outdated --format json`), agrupados por TFM quando houver mais de um.
- Atualização de um pacote por vez: a UI mostra o comando exato e o plano (`dotnet add <csproj> package <id> --version <v> --no-restore`), exige confirmação e, ao terminar, oferece restore explícito. Nunca restaura implicitamente.
- CPM (`Directory.Packages.props` / `ManagePackageVersionsCentrally`): a listagem funciona, o update é recusado com mensagem explicando que o MVP não edita props centrais.
- Busca e instalação de pacote novo ficam fora do MVP.
- Falha de rede/offline preserva o log e mostra o erro bruto; ausência de conexão não vira lista vazia.

### Código

- Gutter: ícone de Run no ponto de entrada do projeto de startup e ícone por teste, além do gutter de breakpoint existente. Posições vêm de `documentSymbol` do LSP; quando a posição não é determinável sem ambiguidade, o ícone não aparece (nada é inventado). Ícones são SVG próprios.
- ⌥↩: popup no cursor com quick fixes e intenções retornadas por `codeAction` para a posição/revisão atuais, com prévia de diff e aplicação validada (mesmo fluxo do rename). Sem ação quando o LSP não retorna nada.
- Menu de contexto do editor: Context Actions, Find Usages, Go To Definition, Rename, Run/Debug Test, Git (commit/diff do arquivo) e Copy path. Itens indisponíveis ficam desabilitados com motivo; Refactor This não existe no MVP.
- Rename inline: campo sobre o identificador no cursor (range de palavra), Enter aplica pelo mesmo fluxo de prévia/revalidação do rename existente, Esc cancela. A prévia não modal continua obrigatória; se houver conflito de foco, a prévia vence e o campo fecha.
- Find Usages (⇧F12): lista somente leitura de ocorrências agrupadas por arquivo, com navegação; sem refatoração em massa.

### Contratos de host novos

Todos seguem o dispatcher existente (`method`, `params`, `result`/erro com `details`), os gates `trust_required`/`stale_context` e estados de dados em vez de erro para indisponibilidade. Limites são declarados na resposta.

| Método | Entrada | Saída | Gate |
|---|---|---|---|
| `workspace.files` | — | caminhos relativos, `total`, `truncated` | leitura |
| `search.text` | `query`, `caseSensitive?` | ocorrências por arquivo (linha/coluna/trecho), `truncated` | leitura |
| `lsp.workspaceSymbol` | `query` | `{name, kind, containerName, uri, range}`, `total`, `truncated`, `state` | análise pronta |
| `lsp.documentSymbol` | `path` | símbolos com range/kind, `state` | análise pronta |
| `lsp.references` | `path`, `line`, `character`, `revision` | localizações dentro do workspace, `rejected`, `state` | análise pronta |
| `git.status` | — | branch, upstream, ahead/behind, arquivos com status | trust |
| `git.diff` | `path`, `staged?` | diff unificado ou lado a lado textual | trust |
| `git.branches` | — | locais e remotas com upstream | trust |
| `git.branchCreate` | `name`, `checkout` | branch atual | trust |
| `git.checkout` | `name` | branch atual | trust + árvore limpa |
| `git.log` | — | até 200 commits com parents | trust |
| `git.commit` | `paths[]`, `message`, `amend` | commit criado (hash, resumo) | trust |
| `git.pull` | — | resultado observado | trust + árvore limpa |
| `git.push` | `confirm:true` | remote/branch e resultado observado | trust |
| `git.path` | — | caminho do executável Git resolvido no PATH, sem iniciar processo | nenhum (metadado local; não abre repositório) |
| `settings.read` / `settings.write` | objeto de settings | settings efetivas | nenhum / nenhum |
| `runConfigs.read` / `runConfigs.write` | solução | lista + id ativo | nenhum / nenhum |
| `nuget.planUpdate` / `nuget.applyUpdate` | projeto, id, versão / `planId` | plano (comando, versão, origem) / resultado + restore pendente | trust |

`settings` e `runConfigs` não executam nada; por isso não exigem trust. Todos os demais que tocam repositório, análise ou rede exigem trust. Nenhum método novo instala ferramenta, altera `global.json` ou faz restore implícito.

### Regras transversais

- `data-testid` existentes são contrato: a casca pode mudar o DOM, não os nomes. Cada fatia adiciona seus testids e mantém `role="tab"` no cabeçalho inferior.
- UI em inglês; documentação, specs e decisões em PT-BR.
- Estados honestos sempre: `stale`, `sealed`, `partial`, `unknown`, `truncated` e "não verificado" não podem virar sucesso, zero ou lista vazia.
- Nenhuma dependência nova (sem Zustand, cmdk, simple-git, lucide ou similar); ícones SVG próprios e estado na view existente.
- Nenhuma telemetria, upload ou serviço remoto obrigatório; logs permanecem locais.
- Windows: contrato e caminhos multiplataforma, execução só no Mac nesta fase; release continua bloqueado por Windows/Intel/packaging (`release-decision.md`).

## Testing Decisions

- **Seam principal: UI desktop (Playwright) → host HTTP de desenvolvimento → ferramentas reais.** É o mesmo seam dos tickets 01–18 e o mais alto disponível: prova a interação e o contrato juntos. Não criar seam novo de UI; os métodos novos de host entram por ele.
- Teste bom prova comportamento externo: texto visível, papel acessível, `data-testid` estável, estado observável e arquivo/processo real. Não testa estrutura interna de componente nem chamadas de função.
- Uma spec Playwright nova por fatia, com a suíte inteira verde antes do commit. Specs existentes não são reescritas; os 1.326 cliques continuam valendo.
- Unit tests de host apenas para lógica pura e parsing novo: `--porcelain=v1 -z`, `git log` com separadores, JSON do NuGet, `.env`, resolução de atalhos e precedência de perfis, caps/truncamento de símbolos e busca.
- Fixtures: a solução `OrderDesk` continua a base. Git usa um repositório temporário descartável por teste (inicializado no próprio teste, nunca o repo do app). NuGet: parsing e recusa de CPM rodam offline; o teste que consulta o nuget.org é opt-in (`OPENRIDR_ENABLE_NETWORK_TESTS=1`) e registra a lacuna quando não roda.
- Testes negativos de trust para todo método novo que executa algo (`git.*`, `nuget.*`), exceto metadado neutro `git.path`: chamada direta em Restricted mode devolve `trust_required` e nenhum processo nasce.
- Testes de estado honesto: busca sem LSP, Git sem repo, Settings inválidas, NuGet offline, CPM, árvore suja no checkout/pull, descoberta stale no rerun failed.
- Workers = 1 e `beforeEach`/`afterEach` restaurando arquivos, como no resto da suíte; o ambiente usa `OPENRIDR_HOME` isolado.

## Out of Scope

Split/detach/reordenar tool windows, slot direito, Notifications, Services, Structure, Bookmarks, TODO, Hierarchy, Endpoints, Database, Coverage/profilers, Continuous Testing, blame inline, Code With Me, Settings Sync, Refactor This, "teste atual no cursor", busca/instalação de pacote NuGet, update em CPM, force push, auto-stash, tema claro, settings por projeto, histórico de navegação (voltar/avançar), abrir em split, MTP, Linux, execução no Windows nesta fase, empacotamento/assinatura/notarização.

## Further Notes

- **Seam a confirmar:** a proposta é manter um único seam (UI → host → ferramentas). Se Helder preferir separar Git/NuGet em testes de contrato puros, a decisão muda os tickets, não o comportamento.
- Riscos conhecidos: volume/latência de `workspace/symbol` no csharp-ls (medir antes de fixar caps); `git commit --only` com arquivo parcialmente staged (testar com fixture real); foco entre rename inline e prévia não modal; acordes ⌘U no perfil IntelliJ (a ação precisa de tecla própria nos dois perfis); Windows não executado.
- Nada de ícones, capturas, textos ou binários do Rider entram no produto; `rider-obs/` é referência interna.
- Próximo passo: `/to-tickets` sobre esta spec, um ticket por fatia (ou subdivisão quando a fatia passar de um ciclo de revisão), com o grafo de dependências acima. Não iniciar implementação antes disso.
- Registro da entrevista e a tabela D1–D16: `~/.opencode/plan/openridr-ui/GRILL-MVP.md`; decisões resumidas em [decisions.md](decisions.md).
