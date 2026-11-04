# SPEC — Paridade com o Rider (gaps G01–G23)

Data: 04/11/2026. Branch de trabalho: `feat/rider-parity`, a partir de `docs/rider-gaps`.
Fontes: `docs/RIDER-BEHAVIORS.md`, `docs/GAPS-RIDER-RIDR.md` e `docs/RETRO-RIDER-GAPS.md`.
Tickets: `docs/tickets-rider-parity/` (01–16). Decisões: `docs/decisions.md`, entrada
"2026-11-04 — Paridade com o Rider".

## Problem Statement

O OpenRidr já cobre o fluxo principal de abrir, editar, buildar, rodar e depurar uma solução
C#, mas a comparação ao vivo com o Rider capturou 23 diferenças que quebram memória muscular
e escondem capacidades: ações que dependem de análise terminam num aviso sem saída; faltam
atalhos para Search Everywhere e para as janelas de ferramenta; a descoberta de testes exige
um clique; resultados de teste e erros de build não levam ao arquivo; menus de solução e de
pasta são mais pobres; não há "Mostrar no Finder"; a configuração de execução derivada é
somente leitura; um breakpoint em linha em branco é aceito em silêncio; e a chrome (rótulo do
keymap, janelas ausentes, ícones) não explica o que é o quê.

O usuário quer fechar esses gaps na ordem P0 → P1 → P2, com critérios verificáveis, sem
regredir Play e Debug, sem dependência nova e sem material proprietário do Rider.

## Solution

Fechar primeiro o bloqueio de análise (G01), depois os gaps P1 que devolvem fluxos centrais
(atalhos, descoberta automática, links de teste, menus, links de build) e por fim os P2 de
polimento e decisão (foco, contador do Find, próxima falha, breakpoint, chrome). Itens de
esforço L ou que exigem backend novo ficam em tickets explícitos marcados `adiado`, com
motivo e o que falta, em vez de desaparecerem.

Cada ticket é verificado no app desktop real com `agent-desktop`; o Playwright novo fica
reservado a contratos de host e regressões estáveis. Nenhum ticket é considerado pronto sem
evidência no app real.

## User Stories

1. Como pessoa usando um workspace confiável sem análise instalada, quero que Find Usages me
   ofereça instalar ou iniciar a análise ali mesmo, para não precisar procurar a tela de
   ferramentas.
2. Como pessoa que acabou de instalar a análise, quero que a consulta de Find Usages pendente
   rode sozinha quando a análise ficar pronta, para não repetir o gesto.
3. Como pessoa usando Rename ou code actions sem análise, quero o mesmo aviso acionável, para
   que o bloqueio tenha uma saída em qualquer uma dessas ações.
4. Como pessoa acostumada ao Rider no macOS, quero ⇧F12 para Find Usages, para que o atalho
   observado no menu do Rider continue valendo.
5. Como pessoa usando o perfil IntelliJ, quero ⌥F7 para Find Usages, para não perder o atalho
   clássico do perfil.
6. Como pessoa navegando por símbolos, quero o atalho do meu perfil documentado e estável,
   para saber qual tecla usar sem adivinhar.
7. Como pessoa acostumada ao Rider, quero abrir Search Everywhere com duplo Shift e com ⌘T,
   para usar o gesto que já conheço.
8. Como pessoa que usa as janelas de ferramenta, quero ⌘1..⌘8 para abrir, focar e fechar
   Solution, Commit, Build, Run, Problems, Debug, Terminal e Tests, para alternar sem mouse.
9. Como pessoa que abre um arquivo pela busca, quero o foco ir para o editor, para que ⌘F
   abra a busca do editor na primeira tentativa.
10. Como pessoa que clica num resultado de teste com stack trace, quero clicar em
    `arquivo:linha` e abrir o arquivo naquela linha, para ir direto ao ponto da falha.
11. Como pessoa revisando testes falhos, quero um botão "Next failure" que percorra as falhas,
    para não caçar cada resultado na lista.
12. Como pessoa abrindo uma solução com projeto de teste, quero a descoberta de testes
    automática, para ver a árvore de testes sem clicar em "Discover tests".
13. Como pessoa cuja descoberta falhou, quero a causa visível no painel, para saber se falta
    SDK, restore, avaliação ou adaptador.
14. Como pessoa usando o menu de uma pasta, quero Novo arquivo, Copiar caminho, Find in Files,
    Renomear, Excluir e Mostrar no Finder, para agir sem sair da árvore.
15. Como pessoa que quer encontrar um arquivo no disco, quero "Show in Finder" em arquivo,
    pasta, projeto e solução, para selecionar o item no Finder.
16. Como pessoa usando o menu da solução, quero Build Solution, Restore Solution, Copy Path e
    Find in Files, para executar as ações centrais sem passar pelo menu Build.
17. Como pessoa vendo "Add Project" no menu da solução, quero o item desabilitado com motivo,
    para não clicar num caminho que não existe.
18. Como pessoa com um projeto de biblioteca selecionado, quero que "Run Unit Test" não
    apareça, para não parecer que o projeto tem testes.
19. Como pessoa com um projeto de teste selecionado, quero "Run Unit Test" rodando todos os
    testes daquele projeto, para executar sem abrir o painel antes.
20. Como pessoa depurando um build com erro, quero clicar em `arquivo(linha,coluna)` no Build
    output e abrir o arquivo na posição, para corrigir sem procurar o caminho no log.
21. Como pessoa usando o Find do editor, quero ver "n de m" ocorrências, para saber onde estou
    e quantas faltam.
22. Como pessoa editando uma configuração de execução derivada, quero um botão
    "Duplicate to edit", para criar uma cópia editável em um clique.
23. Como pessoa usando os chords de teste, quero vê-los na Settings, para saber que existem
    mesmo sem poder remapeá-los ainda.
24. Como pessoa usando um breakpoint em linha em branco, quero um aviso visível de que não há
    código ali, para não esperar uma parada que nunca vem.
25. Como pessoa lendo a barra de status, quero o nome exato do perfil de keymap ativo, para
    não ver um rótulo ambíguo.
26. Como pessoa passando o mouse nos botões de ferramenta ausentes, quero um tooltip honesto,
    para entender por que Monitoramento, AI Assistant e Database não abrem.
27. Como pessoa usando os ícones da Solution, quero nome acessível e tooltip em cada um, para
    navegar por leitor de tela e por hover.
28. Como pessoa tentando depurar um teste, quero que "Debug selected" continue desabilitado
    com o motivo visível enquanto o attach ao test host não existir, para não acreditar numa
    capacidade falsa.
29. Como mantenedor, quero as decisões de keymap, breakpoint, chords e configuração derivada
    registradas com a fonte, para que a próxima pessoa entenda o porquê.
30. Como mantenedor, quero tickets explícitos para o que ficou adiado, para que o backlog não
    perca o item.
31. Como verificador, quero cada critério de aceite provado no app desktop real, para que a
    paridade não seja só uma mudança de DOM.
32. Como mantenedor do repositório público, quero que nada de screenshot, código ou string
    copiada do Rider entre no git, para preservar a licença MIT e o material de terceiros.

## Implementation Decisions

### Escopo e prioridade

A lista fechada é G01–G23 de `docs/GAPS-RIDER-RIDR.md`. Observações dos outros docs que não
viraram gap são contexto e não entram nesta onda.

| Prioridade | Gaps |
|---|---|
| P0 | G01 |
| P1 | G02, G03, G04, G08, G09, G10, G13, G14, G15, G20 |
| P2 | G05, G06, G07, G11, G12, G16, G17, G18, G19, G21, G22, G23 |

Adiados com ticket próprio: G10 (debug de teste), Add Existing Item, Move e Exclude da G16.
Adiados dentro do ticket: "na seleção" da G07, "Open with" da G14, Rebuild/Clean da G15 e
remapear chord da G11.

### Análise e Find Usages (G01)

O aviso de bloqueio de análise vira acionável e é compartilhado por Find Usages, Rename e
code actions. Quando a ferramenta não está pronta, a ação é "Install analysis": abre o painel
de ferramentas e solicita o plano de instalação, que continua exigindo confirmação explícita;
nada é baixado sem esse passo. Quando a ferramenta está pronta mas a análise está parada, a
ação é "Start analysis". Quando a análise fica ativa, a última ação pendente roda uma vez
sozinha e o resultado aparece normalmente. Em modo restrito, o bloqueio continua sendo o
trust, sem oferecer instalação como saída.

### Keymap (G02, G03, G04, G05)

O perfil `rider-macos` mantém ⇧F12 para Find Usages e o perfil `intellij` mantém ⌥F7. O menu
do Rider observado ao vivo mostra ⇧F12, então não há troca. O símbolo continua ⌥⌘⇧T no
`rider-macos`; o mapeamento de Ctrl+Shift+Alt+N do keymap base para macOS não foi confirmado
e fica registrado como pendência, em vez de virar um atalho de quatro modificadores não
verificado.

Search Everywhere ganha duplo Shift e ⌘T nos dois perfis; o atalho aparece na Settings e na
dica da tela inicial do editor. As janelas de ferramenta ganham ⌘1..⌘8 na ordem da faixa
(Solution, Commit, Build, Run, Problems, Debug, Terminal, Tests); Git e NuGet não têm padrão,
mas são remapeáveis. O mesmo atalho alterna: abre, foca e fecha.

Os chords ⌘U+R/D/L/F continuam fixos. A Settings mostra os chords e uma nota de que não são
remapeáveis; um override por atalho único continua funcionando. Editar o chord em si fica
adiado.

### Testes (G08, G09, G12, G17)

A descoberta roda automaticamente uma vez por fronteira de workspace, contexto e revisão de
restore, para o projeto de teste selecionado, quando o modelo avaliado está pronto. A falha
aparece no painel com a causa; não há laço a cada edição. Trocar o projeto selecionado dispara
a descoberta daquele projeto.

A posição `arquivo:linha` da stack trace de um resultado vira link com esse texto e abre o
arquivo na linha. A stack trace crua continua visível; quando não há posição dentro do
workspace, o texto honesto atual permanece.

"Next failure" cicla as falhas, expande o detalhe, rola até o resultado e navega para a origem
quando existe. Não ganha atalho novo.

Em projeto que não é de teste, "Run Unit Test" desaparece. Em projeto de teste, fica
habilitado para rodar todos os testes daquele projeto, com os mesmos gates do painel Tests;
se o host recusar, a mensagem aparece.

### Solution e menus (G13, G14, G15)

O menu de pasta é reproduzido antes de qualquer mudança: o código atual já monta Novo arquivo,
Copiar caminho, Find in Files, Renomear e Excluir. Se abrir no app real, o ticket vira prova e
ganha "Show in Finder"; se não abrir, o handler é corrigido.

"Show in Finder" vira uma operação de host com validação de contenção no workspace, sem gate de
trust, porque abrir o Finder não executa código do projeto. No macOS usa `open -R`; em
plataforma não suportada devolve erro claro em vez de fingir sucesso. Cobre arquivo, pasta,
projeto e solução. "Open with" fica adiado: enumerar aplicativos exigiria dependência nova.

O menu da solução ganha Build Solution, Restore Solution, Copy Path, Find in Files e Add
Project desabilitado com motivo. Rebuild/Clean ficam adiados por exigirem operações novas no
host; não existe item sem backend.

### Build output (G20)

O host devolve spans de localização do log, com linha do log, coluna inicial/final, caminho,
linha e coluna do código, produzidos pelo mesmo parser conservador que já alimenta
`build.problems`. A UI só renderiza botões nesses spans; não reparseia MSBuild. Linha não
reconhecida continua texto. O prefixo de stderr e o log truncado não podem deslocar um link
para a linha errada.

### Navegação e Find do editor (G06, G07)

Abrir arquivo por Search Everywhere, Find in Files, recentes, Find Usages, definição ou link
de teste devolve o foco ao editor depois que ele monta. Clique simples na árvore mantém o foco
na árvore. O painel de Find do editor ganha o contador "n de m", atualizado com a consulta e o
documento. O escopo "na seleção" fica adiado dentro do ticket 08, com motivo e o que falta.

### Configuração derivada e chords (G11, G18)

A configuração derivada continua somente leitura, mas o aviso ganha "Duplicate to edit", que
usa o duplicate existente e deixa a cópia editável. Edição direta exigiria um store de
overrides por solução/contexto; a decisão registra por que o modelo atual fica.

Os chords continuam como estão; a Settings os exibe com a nota de que não são remapeáveis.

### Breakpoint em linha sem código (G19)

O breakpoint é aceito, mas marcado como "sem código nesta linha" (visual atenuado + tooltip)
até o debugger reportar um bound breakpoint. Não recusamos nem movemos para a linha seguinte
às cegas, porque não há mapa confiável de linhas executáveis e o comportamento do Rider nesse
caso continua não verificado. A pendência fica em `docs/decisions.md`.

### Chrome (G21, G22, G23)

A barra de status passa a mostrar o nome exato do perfil ativo, não "IntelliJ / Rider". Os três
botões de ferramenta ausentes mantêm o tooltip que explica que não fazem parte do OpenRidr. Os
ícones da Solution já têm `aria-label` e `title`; o ticket só prova e fecha a regressão.

### Verificação e dependências

Aceite no app desktop real via `agent-desktop`; Playwright novo só para contrato de host ou
regressão estável. O que não rodar entra como "não verificado" com motivo. A branch
`fix/debug-dotnet-path` não é importada, mesclada nem cherry-picked; `child_env.rs` e os
arquivos de ambiente do Debug não são tocados. Falha de Debug pelo sintoma que aquela branch
corrige fica bloqueada por ela, sem contorno.

Sem dependência nova; sem screenshots, código ou strings copiadas do Rider; MIT. Commits só
locais, um por ticket, com `scripts/commit-only.sh`; sem push, merge, PR ou release.
`DESIGN.md` e `PRODUCT.md` não são tocados.

## Testing Decisions

Um bom teste aqui observa comportamento externo: o que a pessoa vê e faz, não a forma como o
componente guarda estado. Onde o host tem contrato próprio, o teste é Rust no host; onde a
regressão é de UI estável, é Playwright contra o host real; o aceite de cada ticket é uma
passada no app desktop real.

- **Host (Rust):** validação e argv de `workspace.reveal` (caminho dentro/fora do workspace,
  plataforma não suportada), spans de localização do log de build (linha reconhecida, linha
  não reconhecida, stderr, log truncado). Prior art: testes de `build.rs`, `logbuf.rs` e
  `crates/host/tests/host.rs`.
- **UI (Playwright):** keymap e Settings, abertura de Search Everywhere, menus da solução e da
  pasta, descoberta automática (com fixture real), link de stack trace, contador do Find,
  "Next failure", "Duplicate to edit", rótulo da status bar. Prior art:
  `tests/e2e/medium-nav-*.spec.ts`, `find-usages.spec.ts`, `run-configs.spec.ts`,
  `build.spec.ts`, `tests.spec.ts`.
- **Desktop (agent-desktop):** cada ticket tem um roteiro curto no próprio ticket; a evidência
  vai para `docs/evidence/rider-parity.md` com data, app e o que foi visto. Finder, foco real,
  hover/tooltip e gutter são casos de desktop, não de navegador.
- **Baseline conhecido:** as falhas listadas em `docs/RETRO-DEVFLOW-2026-11-02.md` e no
  `AGENTS.md` não são culpa desta onda; qualquer outra falha é investigada.
- **Play e Debug verdes:** os specs existentes de execução e depuração continuam passando; se
  um deles falhar pelo ambiente de `fix/debug-dotnet-path`, o ticket fica bloqueado por ela.

## Out of Scope

- G10 (debug de teste com attach ao test host), Add Existing Item, Move e Exclude da G16:
  tickets adiados, sem código nesta onda.
- "na seleção" do Find, "Open with", Rebuild/Clean e remapear chord: adiados com motivo.
- Tradução da UI para pt-BR; a UI continua em inglês.
- Windows, macOS Intel e Linux; o alvo verificado continua macOS Apple Silicon.
- Embutir SDK .NET, trocar o csharp-ls ou mexer no fluxo de instalação revisável.
- Qualquer material proprietário do Rider; as descrições são com palavras próprias.
- `DESIGN.md` e `PRODUCT.md`, que não são rastreados e não podem ser tocados.
- Merge, push, PR, release ou importação de `fix/debug-dotnet-path`.

## Further Notes

Rastreabilidade gap → ticket:

| Gap | Ticket | Gap | Ticket |
|---|---|---|---|
| G01 | 01 | G12 | 10 |
| G02, G03, G04, G05 | 02 | G13, G14, G15 | 05 |
| G06 | 07 | G16 (Add/Move/Exclude) | 14, 15, 16 |
| G07 | 08 | G17 | 10 |
| G08 | 03 | G18, G11 | 09 |
| G09 | 04 | G19 | 11 |
| G10 | 13 (adiado) | G20 | 06 |
| | | G21, G22, G23 | 12 |

Pendências de confirmação registradas: o mapeamento do keymap base para macOS (G02/G05) e o
comportamento do Rider para breakpoint em linha em branco (G19). Nenhuma das duas bloqueia a
entrega; ambas ficam em `docs/decisions.md`.

A ordem de implementação é a dos tickets (P0 → P1 → P2), com um commit local por ticket. O
retro e a evidência desta onda ficam em `docs/RETRO-RIDER-PARITY.md` e
`docs/evidence/rider-parity.md`, criados na etapa de verificação/retro.
