# SPEC — Debug: explicar a sessão que termina sem parar

Issue-mãe: [#46](https://github.com/Heldinhow/openridr/issues/46) — "Debug: explain when a session ends without breakpoints".

## Problem Statement

Um desenvolvedor clica em Debug sem ter marcado breakpoint nenhum (ou com os breakpoints mutados). O programa roda até o fim e o painel Debug termina mostrando apenas **Terminated** e "The debug session terminated", sem threads, frames ou locals. Não há nada no painel dizendo o que aconteceu de fato, então o usuário lê o resultado como falha do depurador e vai procurar defeito onde não há.

O painel hoje só sabe descrever o *estado* do fim; ele não sabe dizer *por que* a sessão nunca parou.

## Solution

Quando a sessão termina **sem ter parado nenhuma vez**, o painel Debug passa a explicar o fim em uma frase própria, acima das listas de threads/frames: o programa rodou até o fim, com o exit observado (quando houve) e o ponteiro para a saída do programa na aba Console. Se havia breakpoints mas todos estavam mutados, a frase diz isso em vez de afirmar que nenhum breakpoint foi setado. Nenhuma das duas frases aparece quando a sessão parou ao menos uma vez, quando a parada foi pedida pelo usuário, quando o depurador falhou, ou quando o usuário configurou outro motivo de parada (break on thrown exceptions, run-to-cursor).

## User Stories

1. Como desenvolvedor que apertou Debug sem breakpoint, quero que o painel diga que o programa rodou até o fim, para não concluir que o depurador quebrou.
2. Como desenvolvedor que esqueceu o breakpoint, quero a explicação do fim, para saber o que fazer na próxima tentativa.
3. Como desenvolvedor que parou num breakpoint e depois deu Continue, quero que o painel **não** afirme que nenhum breakpoint foi setado, para o texto não contradizer o que acabei de ver.
4. Como desenvolvedor que mutou todos os breakpoints antes de rodar, quero que a frase diga que eles estavam mutados, para a explicação bater com a minha ação.
5. Como desenvolvedor que ligou "break on thrown exceptions" e o programa não lançou nada, quero continuar sem a frase de "nenhum breakpoint setado", para o texto não mentir sobre o que configurei.
6. Como desenvolvedor que pediu run-to-cursor e o programa terminou antes, quero que o painel não diga "rodou até o fim sem parada configurada", para o texto não apagar o pedido que fiz.
7. Como desenvolvedor lendo o painel, quero o exit observado dentro da frase, para saber se o programa terminou bem.
8. Como desenvolvedor cuja sessão terminou sem exit reportado, quero a frase **sem** número inventado, para o painel não afirmar um exit que ninguém observou.
9. Como desenvolvedor, quero que a frase aponte para a aba Console, para ler o que o programa imprimiu sem procurar.
10. Como desenvolvedor que clicou em Stop, quero o fim descrito como parada minha, e não como "rodou até o fim".
11. Como desenvolvedor cuja sessão falhou (erro do adaptador), quero ver o texto da falha, e não a explicação de "sem breakpoint".
12. Como usuário de leitor de tela, quero a explicação como texto do painel (não tooltip, não título), para poder lê-la na ordem normal do documento.
13. Como usuário de leitor de tela, quero que a explicação não duplique o anúncio da região de status do painel, para não ouvir o mesmo fim duas vezes.
14. Como desenvolvedor que usa Restart, quero que a explicação pertença só à sessão nova, porque o estado de "parou alguma vez" é da sessão.
15. Como desenvolvedor que lança outra vez sem breakpoint, quero a mesma explicação de novo, e não uma nota velha reaproveitada.
16. Como desenvolvedor, quero a frase **substituindo** a nota genérica de fim, e não somada a ela, para não ler "a sessão terminou" duas vezes.
17. Como mantenedor, quero a regra calculada uma vez no host, para que o painel e qualquer outro leitor do snapshot não possam discordar.
18. Como mantenedor, quero a tabela-verdade presa por testes de host com adaptador controlado, para provar os casos sem depender do netcoredbg instalado.
19. Como mantenedor, quero os casos positivos e negativos provados no E2E contra o host real e a fixture, para não confiar só no teste unitário.
20. Como testador, quero ver a frase no app real (Tauri) com a fixture copiada, para provar que o painel renderiza o texto e mantém o output visível.
21. Como desenvolvedor, quero que a aba escolhida por mim continue escolhida depois do fim da sessão, porque a explicação aponta para o Console e não deve roubar a navegação.
22. Como desenvolvedor em sessão selada (troca de workspace), quero continuar vendo a explicação da sessão antiga junto com o aviso de "sessão anterior", para entender o que aquela sessão fez.
23. Como desenvolvedor, quero que o painel continue sem threads, frames e locals nesse caso, porque a sessão realmente não tem pausa — a frase é a informação que faltava, não um substituto inventado.

## Implementation Decisions

- **Dono da regra: o host.** O fim de sessão já passa por um único ponto (`Data::finish`); é lá que a explicação é derivada dos fatos da sessão e viaja no snapshot. O React só renderiza o campo — não recalcula estado, breakpoints nem mute. Vale a decisão de 2026-11-05 ("A UI não recalcula regra") e o precedente da frase de "infrastructure failure" do test runner.
- **Contrato do snapshot.** `DapView` ganha `noStopExplanation: string | null` (novo; nenhum campo existente muda de nome ou some). `null` significa "nada a explicar"; a presença do texto é a única condição que o painel usa.
- **"Parou alguma vez" é um fato próprio.** Um booleano por sessão, marcado no primeiro evento `stopped` do adaptador (breakpoint, exceção, pause ou run-to-cursor) e zerado em cada lançamento. Continuações não apagam o fato: parar e depois continuar ainda conta como ter parado. O contador de geração de pausa não serve para isso, porque ele também avança em cada resume.
- **Elegibilidade por estado.** Só `terminated` explica. `cancelled` mantém o motivo da parada do usuário; `failed` mantém o texto da falha; `idle`/`building`/`running`/`paused` não têm explicação de fim.
- **Motivos de parada conhecidos.** Contam como "algo foi configurado para parar": breakpoints armazenados (mesmo não verificados), o filtro de exceções lançadas e o run-to-cursor pendente. Com qualquer um deles a frase de "nenhum breakpoint" não aparece.
- **Mute tem frase própria.** Breakpoints armazenados com mute-all ligado produzem a variante mutada; sem mute, breakpoints armazenados e nunca atingidos **não** produzem explicação nesta rodada (ver Out of Scope).
- **Texto literal (inglês, como o resto da UI):**
  - `No breakpoints were set; the program ran to completion (exit {N}). Its output is in the Console tab.`
  - `Breakpoints are muted; the program ran to completion (exit {N}). Its output is in the Console tab.`
  - Sem exit observado, o parêntese do exit desaparece: `… the program ran to completion. Its output is in the Console tab.`
- **Onde aparece no painel.** Um parágrafo próprio, com classe de informação (não atenuada), logo abaixo da linha de estado/fase e acima das colunas de Threads, com `data-testid="dap-no-stop"`. Nesse caso ele **substitui** a nota atenuada de fim de sessão; nos outros fins a nota atenuada continua igual. Nenhum `data-testid` existente é renomeado ou removido.
- **Sem troca de aba.** A explicação aponta para o Console, que já é a aba padrão e permanece montada; o painel não muda a aba escolhida nem rola nada.
- **Acessibilidade.** O parágrafo é texto normal dentro da seção do painel; a região de status continua sendo só o selo de estado, para o leitor de tela não anunciar o fim duas vezes.

## Testing Decisions

- **O que faz um bom teste aqui:** ele olha o comportamento observável — o texto no snapshot e o texto no painel —, nunca os campos internos que derivam a decisão. Um teste que ligasse breakpoints e afirmasse um booleano interno passaria com a regra errada.
- **Host (unitário, `crates/host/src/dap.rs`):** montar a sessão com o coletor de escrita já existente e dirigir o ciclo por mensagens DAP reais (`exited`, `stopped`, `continued`, `terminated`), como fazem os testes de exit desconhecido e de geração de pausa. Casos: (a) nunca parou e sem breakpoints → frase sem-breakpoint com o exit; (b) exit nunca reportado → frase sem parêntese de exit; (c) parou e continuou → sem frase; (d) breakpoints setados e não mutados → sem frase; (e) breakpoints mutados → frase de mute; (f) filtro de exceções lançadas ligado → sem frase; (g) encerramento pedido pelo usuário (`cancelled`) → sem frase.
- **E2E (`tests/e2e/debugger.spec.ts`), host real + fixture:** o caminho positivo já é montável pelo helper que prepara a cópia da fixture com breakpoint opcional: lançar sem breakpoint, esperar `Terminated`, exigir a frase e a ausência da nota atenuada, e exigir a saída do programa na aba Console. Os dois caminhos negativos aproveitam specs que já existem: a condição que nunca vale (breakpoint setado, sem frase) e o mute-all (frase de mute, substituindo a de "nenhum breakpoint").
- **App real:** verificar com `agent-desktop` no app Tauri sobre uma cópia da fixture em `/tmp` — uma instância, fechada ao final. O que o ambiente não permitir entra como "não verificado".

## Out of Scope

- "Breakpoints setados mas nunca atingidos": breakpoint não verificado, condição falsa ou filtro de exceção que nunca dispara continuam sem explicação própria (issue separada, se algum dia for pedida).
- Qualquer mudança em como a sessão começa: attach, test-host, launch profiles, gates de debug.
- Trocar de aba, rolar o output, ou reorganizar a ordem dos painéis de threads/frames/locals.
- Reaproveitar a explicação entre sessões ou entre workspaces: ela pertence à sessão que terminou, inclusive depois de selada.

## Further Notes

- A issue não pede mensagem em `cancelled`/`failed`, e "rodou até o fim" seria falso nos dois.
- A frase fica visível mesmo depois de a sessão ser selada por troca de workspace, junto do aviso de "sessão anterior": é o registro do que aquela sessão fez.
- Seams: nenhum seam novo. O mais alto já existe — o snapshot do host (`workspace.state` → `dap`) para o valor, e o DOM do painel (`dap-no-stop`) para o texto. É o mesmo par usado pelos specs de Debug de hoje.
