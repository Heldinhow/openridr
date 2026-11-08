# SPEC — E2E com portas por sessão e Playwright como verificação de UI (issue #210)

Status: ready-for-agent
Fonte: issue GitHub #210 (Heldinhow/openridr) + grill de 2026-11-07 com o orquestrador (Q1–Q6) + decisão do Helder de 2026-11-07 sobre a superfície de verificação de UI.
Tickets: issues filhas de #210 (criadas por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

Duas sessões de agente não conseguem rodar E2E ao mesmo tempo, e o E2E não convive com `bun run dev`. Metade do problema já foi resolvida: as portas do harness vêm de `OPENRIDR_E2E_UI_PORT`/`OPENRIDR_E2E_HOST_PORT`, com 1420/4317 como default, e o Vite de teste fala com o host pelo proxy same-origin `/__host`. O que sobrou é o **E2E app home**: ele é um caminho fixo no diretório temporário, igual para todos os worktrees, e o bootstrap de cada execução reescreve as ferramentas verificadas dentro dele. Duas execuções simultâneas — em worktrees diferentes ou não — brigam pelo mesmo diretório: `settings.json`, o launcher do csharp-ls e o adaptador embutido de debug são apagados e recriados embaixo da outra execução. Portanto o critério "duas worktrees ao mesmo tempo" não é verdade hoje, mesmo com portas diferentes.

Em paralelo, a documentação do processo ficou para trás. `AGENTS.md` e o skill `verify-openridr` afirmam que Playwright é **só** smoke noturno e **nunca** verificação de UI por sessão. A decisão do Helder de 2026-11-07 mudou isso: mudança de interface pura se verifica com Playwright em modo web, e o app Tauri real dirigido por `agent-desktop` fica para o que é nativo do shell. Dois outros arquivos (`scripts/e2e-smoke.txt` e `docs/evidence/e2e-quarantine.md`) repetem a versão antiga. Quem lê o repositório hoje encontra duas regras contraditórias e escolhe a que vier primeiro.

## Solution

1. O E2E app home passa a ser derivado da porta da UI: `openridr-e2e-home-<porta>` no diretório temporário. Portas diferentes são homes diferentes, e duas execuções na mesma porta não podem coexistir de qualquer forma (o Vite falha rápido com `strictPort`). `OPENRIDR_E2E_HOME` continua vencendo como override explícito. Com isso, o mesmo spec roda em dois worktrees ao mesmo tempo, cada um com seu par de portas.
2. `AGENTS.md` e o skill `verify-openridr` descrevem a divisão de superfícies: Playwright em modo web para mudança de interface pura (escrevendo/atualizando o spec do cenário), app Tauri real com `agent-desktop` só para o que é nativo do shell, host isolado para o resto. A convenção de portas (default e faixa de uma sessão paralela) entra junto, e o texto que proibia rodar E2E junto com `bun run dev` sai.
3. Os dois arquivos que repetem a regra antiga são corrigidos, e a decisão de 2026-11-07 fica registrada em `docs/decisions.md`, para a próxima sessão não desfazer por engano.

## User Stories

1. Como sessão de agente, quero rodar um spec Playwright no meu worktree enquanto outra sessão roda o mesmo spec no worktree dela, para não esperar a vez.
2. Como sessão de agente, quero que a única coisa que eu precise escolher sejam duas portas livres, para não descobrir um terceiro estado compartilhado depois de 10 minutos de suíte.
3. Como sessão de agente, quero que o harness falhe na hora se a porta que eu escolhi estiver ocupada, para não rodar contra o Vite de outra pessoa.
4. Como sessão de agente, quero poder rodar E2E com `bun run dev` aberto no mesmo worktree, escolhendo portas diferentes das do dev, para verificar UI sem derrubar meu ambiente.
5. Como sessão de agente, quero continuar podendo apontar `OPENRIDR_E2E_HOME` para um diretório meu, para investigar um estado sujo sem perder o default.
6. Como mantenedor, quero que o smoke noturno continue rodando sem nenhuma variável nova, para o workflow não virar mais um lugar onde a configuração mora.
7. Como mantenedor, quero que duas execuções do smoke nunca se sobreponham, para o home de um run não ser reescrito pelo outro (o workflow já tem `concurrency` sem cancelamento).
8. Como sessão de agente que mudou só interface, quero que o `AGENTS.md` diga para verificar com Playwright em modo web, para não gastar 20 minutos abrindo o app real para uma mudança de CSS.
9. Como sessão de agente que mexeu em menu nativo, atalho do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance ou DMG, quero que o `AGENTS.md` diga para usar o app real com `agent-desktop`, para não "provar" uma superfície que o navegador não tem.
10. Como sessão de agente, quero saber **como** escolher as portas (default e faixa alternativa) na mesma seção em que leio o comando, para não inventar um par que colide.
11. Como sessão de agente, quero saber que uma sessão que usa as portas default precisa pedir o slot ao orquestrador, para não disputar 1420/4317 com outra sessão.
12. Como mantenedor lendo o repositório, quero que `AGENTS.md`, o skill `verify-openridr`, a lista do smoke e a quarentena digam a mesma coisa sobre Playwright, para não haver duas regras contraditórias.
13. Como mantenedor, quero a decisão de 2026-11-07 em `docs/decisions.md` com o motivo e a consequência, para a mudança de processo ter o mesmo rastro das mudanças de produto.
14. Como sessão futura, quero a prova de duas worktrees em paralelo registrada em `docs/evidence/`, com as portas, os comandos e o resultado, para saber que o caminho funciona sem repetir a medição.
15. Como mantenedor, quero que a suíte E2E continue sendo a mesma (mesmos specs, mesmos `data-testid`, nenhuma dependência nova), para esta frente não virar uma segunda frente de testes.

## Implementation Decisions

- **Portas: nomes e defaults ficam.** `OPENRIDR_E2E_UI_PORT` e `OPENRIDR_E2E_HOST_PORT` mantêm o nome, a validação (inteiro de 1 a 65535, recusa de porta inválida e de par igual) e os defaults 1420/4317. Os specs que sobem host próprio herdavam o par escolhido com `hostPort + N`; **atualizado pela #246
  (2026-11-08):** hoje eles pedem porta efêmera ao kernel (`--port 0`) e não dependem do par.
- **E2E app home derivado da porta da UI.** O default passa a ser `<tempdir>/openridr-e2e-home-<uiPort>`; `OPENRIDR_E2E_HOME` continua vencendo. A chave é a porta, não o worktree: duas execuções no mesmo worktree com portas diferentes também precisam de homes diferentes (o fixture é o mesmo, mas o home não pode ser), e duas na mesma porta não coexistem.
- **O cache de ferramentas verificadas é por home, e isso é aceito.** O download/instalação do csharp-ls (~1 min, com rede) roda de novo quando a porta muda. Em troca, nenhuma execução reescreve o que a outra está usando. O smoke noturno usa as portas default, então mantém o mesmo caminho quente.
- **Smoke noturno intocado.** O workflow continua sem setar portas nem home: defaults, `concurrency` já existente. Nenhuma variável nova no CI.
- **Convenção de portas documentada, sem helper.** Default 1420 (UI) / 4317 (host). Sessão que precisa rodar em paralelo, ou junto com `bun run dev`, escolhe outro par (faixa 15xx/44xx) e confere com `lsof -nP -iTCP:<porta> -sTCP:LISTEN`. O `strictPort` do Vite de teste transforma porta ocupada em falha imediata. Nada de script que "acha porta livre". **Atualizado pela #246 (2026-11-08):** as auxiliares deixaram de fazer parte da convenção — cada spec que sobe host próprio usa porta efêmera (`--port 0`) e `tests/e2e/isolated-host.ts` lê a porta concedida da linha que o binário imprime. Só o par do harness é escolhido, e a faixa de 10 portas de host acima dele não existe mais.
- **Sem mudança de produto.** `host-http` e o CORS fixo em 1420 não mudam: o E2E fala com o host pelo proxy same-origin `/__host` do Vite de teste, então a origem do navegador é sempre a do Vite. Nenhum comando de host, nenhum `data-testid`, nenhum CSS.
- **Divisão de superfícies (decisão do Helder, 2026-11-07).** Mudança de interface pura → Playwright em modo web, só os specs da área, escrevendo/atualizando o spec do cenário. Específico do app desktop (menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance, DMG) → app Tauri real com `agent-desktop`, uma instância por vez. O host isolado (`web.sh`) continua para o que não tem spec ou precisa de captura.
- **A regra antiga some de todos os lugares que a repetem**: `AGENTS.md` (comandos, seção E2E, "O que o CI não cobre", passo de verificação), `verify-openridr` (tabela de superfícies e Launch A), `scripts/e2e-smoke.txt` (cabeçalho) e `docs/evidence/e2e-quarantine.md` (cabeçalho). O que continua valendo da #67 é o que a decisão nova não revogou: a quarentena com motivos, o smoke noturno como único E2E automatizado de PR/CI, e "nunca em PR".
- **Sem dependência nova, sem script novo.**

## Testing Decisions

- **Bom teste aqui prova comportamento observável do harness**, não estrutura interna: qual home o host abre, e se duas execuções em paralelo chegam ao fim verdes. Nada de espiar constante de módulo.
- **Seam único (existente): o módulo de ambiente do E2E.** Ele já é o ponto por onde `playwright.config.ts`, o `globalSetup` e todos os specs leem portas, home e URLs; a derivação do home entra ali como função pura, testada no spec que já existe para esse módulo (que hoje valida a leitura de porta).
- **Falha-antes/passa-depois:** o teste unitário novo pina que dois valores de porta dão homes diferentes e que o override vence; o "antes" honesto é a execução real — o mesmo spec em dois worktrees ao mesmo tempo, que hoje disputa o home fixo.
- **Aceitação (o critério da issue):** o mesmo spec rodando em dois worktrees ao mesmo tempo, com pares de portas diferentes, ambos verdes; e uma execução isolada no par default (o caminho do smoke noturno) verde. Os números, comandos e portas entram em `docs/evidence/e2e-ports.md`.
- **Prior art:** o spec de ambiente existente (validação de porta e recusa de origem estrangeira no proxy) e o estilo de medição de `docs/evidence/e2e-quarantine.md` (comando, ambiente, data, resultado).
- **Nada de suíte completa por reflexo:** os specs tocados pela frente são o de ambiente e o escolhido para a prova; o resto da suíte é smoke noturno.

## Out of Scope

- Consertar os specs quarentenados (#43/#67): `debugger`, `csharp-ls:150`, `testsuite`, `tests:317`, `ui-layout` em 1440 px, `find-usages`, `run-configs`, `tests`. Continuam fora do verde.
- Rodar Playwright em PR, ou transformar o smoke noturno em check obrigatório.
- Helper/script que escolhe portas livres; portas aleatórias. Porta `0` estava aqui e saiu pela #246 (2026-11-08): o host auxiliar do E2E sobe em porta efêmera, concedida pelo kernel.
- Mudar `host-http` (CORS, autenticação) ou o proxy do Vite de teste.
- Windows e macOS Intel.

## Further Notes

- **Por que não home por worktree.** Reaproveitaria o cache de ferramentas entre execuções do mesmo worktree, mas duas execuções no mesmo worktree com portas diferentes continuariam brigando pelo mesmo diretório — e é exatamente esse o caso de quem roda E2E com `bun run dev` aberto.
- **O que a porta não separa.** O fixture continua sendo o do worktree (`fixtures/OrderDesk`), e o mesmo worktree não pode rodar dois specs que editam o mesmo arquivo ao mesmo tempo. A separação é entre worktrees, não dentro de um.
- **Custo aceito.** Primeira execução em cada porta nova paga o download/instalação do csharp-ls.
- **Risco do PR.** A frente toca `AGENTS.md`, que é `risk:high`: o merge é do Helder, e a sessão para depois do PR verde.
