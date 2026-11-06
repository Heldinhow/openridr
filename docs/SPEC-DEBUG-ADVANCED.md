# SPEC — Debug: hit count, watches persistentes e verificação real (2026-11-06)

Frente da issue [#47](https://github.com/Heldinhow/openridr/issues/47) ("Debug: conditional
breakpoints, watches and expression evaluation"). Fonte de código no HEAD `7a6c3cb`.

Condição de breakpoint, watches e Evaluate pausado **já existiam** quando esta spec foi escrita:
condição (`debug.setBreakpointCondition`), watches (`debug.addWatch`/`debug.removeWatch`) e
Evaluate pausado com hover já têm contrato, UI e E2E real com netcoredbg. O que falta é o
**hit count**, duas correções de watch e a prova no app desktop real — o "Done when" da issue.

## Problem Statement

Quem depura com o OpenRidr contra o netcoredbg consegue parar por condição, olhar variáveis e
avaliar expressões, mas não consegue parar na N-ésima vez que uma linha é atingida. O hit count
não existe porque o netcoredbg não o implementa e a decisão de 2026-10-30 o deixou de fora; a UI
ainda anuncia essa ausência em texto.

Além disso, a lista de watches é perdida em todo Restart/novo Debug, e os valores mostrados nela
continuam presos ao frame do topo mesmo depois de o usuário escolher outra thread ou frame. A
sensação é de que o watch "não é do usuário": ele digita de novo a cada sessão e vê o valor errado
quando está investigando outra thread.

Por fim, os três recursos nunca foram provados no app Tauri real, só no navegador contra o host
isolado; a issue pede explicitamente o app real na fixture OrderDesk.

## Solution

- **Hit count no host**: cada breakpoint aceita um alvo inteiro N ("parar na N-ésima parada"). Como
  o adaptador ignora `hitCondition`, o host conta as paradas reais, casa o frame do topo com o
  breakpoint e se auto-continua até a N-ésima, quando a pausa é mostrada normalmente. A contagem
  é da sessão e recomeça no launch; condição filtra antes da contagem; Pause manual do usuário
  nunca é engolido; a fase mostra quando o host está retomando por hit count.
- **Watches do usuário**: expressões sobrevivem a Restart e novo launch (como breakpoints, mute e
  filtros), somem só ao fechar/trocar o workspace, e são avaliadas contra a thread/frame
  selecionados, reavaliando ao trocar de frame.
- **Prova no app real**: condição, hit count, watches e Evaluate pausado verificados no bundle
  Tauri dirigido por agent-desktop, contra a fixture OrderDesk copiada para `/tmp`, com evidência
  em `docs/evidence/debug-advanced.md`.

## User Stories

1. Como pessoa depurando, quero definir um hit count num breakpoint existente, para parar só na N-ésima vez que a linha é atingida.
2. Como pessoa depurando, quero ver e editar o hit count ao lado da condição, para ajustar o breakpoint sem abrir outra janela.
3. Como pessoa depurando, quero que o programa rode direto até a N-ésima parada, para não clicar Continue várias vezes.
4. Como pessoa depurando, quero que a barra/painel mostre que o host está contando paradas, para não achar que o app travou entre hits.
5. Como pessoa depurando, quero que um Pause manual funcione mesmo com hit count armado, para poder interromper a contagem.
6. Como pessoa depurando, quero que condições e hit count convivam no mesmo breakpoint, para parar na N-ésima vez em que a condição for verdadeira.
7. Como pessoa depurando, quero que um breakpoint sem hit count continue parando em todo hit, para não mudar o comportamento atual.
8. Como pessoa depurando, quero que um hit count inválido seja recusado com mensagem clara, para não armar um número que nunca para.
9. Como pessoa depurando, quero que o hit count valha para a sessão e recomece no next launch, para o comportamento ser previsível.
10. Como pessoa depurando, quero meus watches preservados ao dar Restart, para não redigitar as mesmas expressões.
11. Como pessoa depurando, quero meus watches preservados ao iniciar um novo Debug, para continuar a investigação.
12. Como pessoa depurando, quero que os watches sumam ao fechar o workspace, para não vazar expressões entre projetos.
13. Como pessoa depurando, quero que um watch mostre o valor no frame que selecionei, para investigar a chamada certa.
14. Como pessoa depurando, quero que um watch acompanhe a troca de thread/frame durante a pausa, para não ler valor de outro contexto.
15. Como pessoa depurando, quero que um watch sem sessão pausada fique pendente sem erro, para poder preparar a lista antes de lançar.
16. Como pessoa depurando, quero ver condição, hit count, watches e Evaluate funcionando no app instalado, para confiar no produto, não só no harness.
17. Como pessoa revisando o produto, quero evidência reproduzível no app real, para saber o que foi e o que não foi verificado.
18. Como pessoa mantendo o host, quero o hit count isolado numa decisão pura e testada, para não depender de relógio/adaptador nos testes.
19. Como pessoa mantendo a UI, quero os `data-testid` antigos preservados e os novos declarados, para os E2E continuarem contrato.
20. Como pessoa mantendo o produto, quero a decisão de 2026-10-30 substituída com o motivo medido, para ninguém reabrir a mesma dúvida.

## Implementation Decisions

- **O adaptador não ajuda e foi medido.** netcoredbg 3.2.0-1092 não anuncia `supportsHitConditionalBreakpoints`, ignora `hitCondition` no `setBreakpoints` e não manda `hitBreakpointIds` no `stopped`. A contagem é responsabilidade do host; `hitCondition` nunca é enviado ao adaptador.
- **Modelo do breakpoint** ganha `hitCount: Option<u32>` (nulo = sem alvo), ao lado de `condition`. O campo é configurável sem sessão e sobrevive ao launch junto com a condição.
- **Contador por sessão**: o host guarda, por sessão, quantas paradas cada `(path, line)` já teve. Chaves são o caminho relativo ao workspace e a linha. O contador zera em todo launch e ao (re)definir o hit count daquele breakpoint.
- **Quando conta**: só `stopped` com `reason == "breakpoint"`. Pausa manual (`pause`), passo, exceção e entrada não contam. O casamento usa o frame do topo da thread parada (caminho canônico relativo ao root + linha); sem casamento, a pausa acontece normalmente e nada é contado.
- **Condição filtra antes da contagem**: o adaptador aplica `condition` e só então o host vê a parada; portanto condição + hit count conta apenas hits aprovados pela condição.
- **Alvo N**: inteiro positivo, "parar quando a contagem for igual a N". Depois de N, o breakpoint não para mais nesta sessão (a contagem continua subindo, sem novo stop). Operadores (`>=`, `%`), log message e `hitCondition` textual não entram.
- **Auto-continue**: ao contar e não atingir N, o host retoma sozinho, sem renderizar a pausa. A fase/estado visível indica a contagem (ex.: `hit count 2/3 · src/Program.cs:14`); o Pause do usuário vale e a parada resultante (`reason == "pause"`) é mostrada. Falha ao retomar encerra a contagem e expõe o erro como hoje.
- **Watch é estado do host**: a lista de expressões é preservada em Restart/novo launch, como breakpoints/mute/filtros, e limpa em fechar/trocar workspace. Não persiste em disco.
- **Watch segue a seleção**: a avaliação usa a thread/frame selecionados, refeita na pausa, ao trocar de thread e ao trocar de frame. Resultados continuam presos à geração de parada (`stopGeneration`), descartando respostas tardias.
- **Evaluate não muda**: continua só de leitura (valor, tipo, expansão, erro); `setExpression`/atribuição ficam fora.
- **Contrato de testes**: a decisão do hit count vira função pura (entrada: breakpoints com alvo, contagem atual, caminho/linha do frame do topo, motivo do stop; saída: parar ou retomar + nova contagem), testada em unidade, sem processo/adaptador. O resto é provado pelo E2E real que já existe em `debugger.spec.ts` e pela verificação no app desktop.
- **`data-testid`**: preservados todos os existentes (`debug-breakpoint-*`, `debug-watch-*`, `debug-evaluate-*`, `debug-breakpoints-unsupported`). Novos: `debug-breakpoint-hit-<path:line>` (input) e `debug-breakpoint-hit-save-<path:line>` (botão). O texto de `debug-breakpoints-unsupported` deixa de citar hit count e passa a citar só log message.
- **Método de host novo**: `debug.setBreakpointHitCount { path, line, hitCount }`, com o mesmo gate de trust dos demais `debug.*` que mexem em breakpoint e o mesmo reenvio da fonte após a mudança.
- **Sem dependência nova**. Sem mudar a decisão de exceções, Run to Cursor, mute ou log message.

## Testing Decisions

- Bons testes aqui provam **comportamento externo**: o snapshot do host e o que o debuggee realmente faz (parou/não parou, valor de variável), nunca campos internos de `Data`.
- **Unidade** (`crates/host/src/dap.rs`): função pura de decisão do hit count — sem alvo, alvo igual, abaixo, acima, motivo diferente de `breakpoint`, caminho fora do workspace, contagem reiniciada. Prior art: os testes unitários existentes no fim de `dap.rs` (parser, framing, gates).
- **E2E** (`tests/e2e/debugger.spec.ts`, seam existente com host real + netcoredbg real + fixture OrderDesk): hit count N para no N-ésimo com o valor esperado em Evaluate; alvo grande deixa terminar (sem stop) ou para uma vez; condição + hit count combinados; contador sobrevive dentro da sessão e zera no Restart; watch sobrevive ao Restart e mostra o valor do frame selecionado após trocar de thread. Prior art: os casos "a conditional breakpoint stops only when its condition is true" e "selects another thread and shows its own frames while paused".
- **Snapshot**: declarar `hitCount` em `DapBreakpoint` e `watches` inalterado; o E2E lê pela UI, não por setters internos.
- **App real** (ticket de verificação): `verify-openridr` superfície C, bundle deste worktree, `OPENRIDR_HOME` isolado, cópia da fixture em `/tmp`, uma instância; nenhuma automação por Playwright para a UI do app.
- Teste instável conhecido (`find-usages`, 2 em `run-configs`, 2 em `tests.spec`) não bloqueia esta frente; qualquer outra falha é regressão.

## Out of Scope

- Log message / logpoints (`supportsLogPoints` também ausente no adaptador).
- `hitCondition` textual enviado ao adaptador ou suporte a múltiplos adaptadores por capability.
- Atribuição por Evaluate (`setExpression`/`setVariable`), hot reload, attach, debug de testes.
- Persistir watches/breakpoints em disco ou por projeto entre execuções do app.
- Diálogo de edição de breakpoint no gutter (o painel Debug continua sendo o dono da edição).
- Windows e macOS Intel: não executados, não declarados suportados.

## Further Notes

- Decisão que substitui a de 2026-10-30 no ponto do hit count: `docs/decisions.md`, 2026-11-06.
- A issue #47 é a mãe; as fatias são GitHub issues filhas (`Part of #47`), a última bloqueada pelas duas primeiras.
- Evidência: `docs/evidence/debug-advanced.md`; retro: `docs/RETRO-DEBUG-ADVANCED.md`.
