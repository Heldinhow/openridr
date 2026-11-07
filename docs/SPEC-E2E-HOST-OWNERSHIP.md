# SPEC — Host auxiliar do E2E com identidade provada (issue #216)

Status: ready-for-agent
Fonte: issue GitHub #216 (Heldinhow/openridr), achado da retro de #210 ([RETRO-E2E-PORTS](RETRO-E2E-PORTS.md)), grill de 2026-11-07 com o orquestrador (Q1–Q4).
Tickets: issues filhas de #216 (criadas por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

Sete specs sobem o próprio host: `trust-prompt`, `welcome`, `medium-editor-session`, `debug-advanced`, `tools-install-ux`, `analysis-gate` e `setup-gate-layout`. Cada um escolhe `hostPort + 1` … `hostPort + 7`, roda `cargo run -p openridr-host-http -- --port N` e faz poll de `/health` até responder. O poll aceita **qualquer** processo que responda naquela porta: se outra sessão (ou um host que sobrou de uma rodada anterior) já é dona da porta, o `cargo run` falha em bindar e ninguém lê esse erro — o spec segue dirigindo o host alheio. O resultado é o pior tipo de teste: verde, mas provando o estado de outro `OPENRIDR_HOME`, com trust, sessão e ferramentas que não são os que o spec preparou. E, quando fica vermelho, a mensagem aponta para o produto em vez da colisão de porta.

O mesmo trecho existe sete vezes: `spawn` + poll, mais o `page.route` que redireciona `/__host/(call|health)` para o host do spec, mais o `mkdtemp` do home e o `kill` + `rmSync` do `afterAll`. Sete cópias de uma regra de segurança é sete lugares para consertar da próxima vez.

## Solution

Um helper único (`tests/e2e/isolated-host.ts`) passa a ser o único jeito de um spec obter host próprio. Ele sobe o host, espera o `/health` e **prova que o processo que respondeu é o que ele subiu**, comparando o `pid` do `/health` com o pid do filho. Porta ocupada por outro processo deixa de ser sucesso silencioso e vira falha imediata com mensagem que nomeia a porta e o pid estranho. O helper também assume o que os sete specs repetiam: o redirecionamento de `/__host` para o host do spec e o ciclo de vida do `OPENRIDR_HOME` temporário.

Os specs continuam escolhendo `hostPort + N` e continuam podendo preparar o `home` (escrever `recent.json`, apontar instalador falso, trocar o adaptador embutido). Nada muda no binário do host, no host compartilhado do harness nem na convenção de portas.

## User Stories

1. Como sessão de agente que roda um spec com host próprio, quero que o spec prove que o host que respondeu é o que ele subiu, para não rodar verde contra o host de outra sessão.
2. Como sessão de agente, quero que porta auxiliar já ocupada falhe na hora com mensagem que nomeia a porta, para saber que o problema é colisão e não o produto.
3. Como sessão de agente, quero a mesma garantia nos sete specs, para não descobrir depois que só um deles era confiável.
4. Como sessão de agente, quero que a mensagem de falha traga o pid estranho, para achar o dono da porta com `ps`/`lsof` sem adivinhar.
5. Como sessão de agente, quero que a falha ao subir o host mate o que subiu e apague o home temporário, para a rodada seguinte não herdar processo nem estado velho.
6. Como sessão de agente, quero que o `afterAll` de cada spec continue matando o host e apagando o home, agora com uma chamada só, para não sobrar lixo entre rodadas.
7. Como mantenedor, quero o par spawn/poll num único lugar, para corrigir a regra de posse uma vez em vez de sete.
8. Como mantenedor, quero o redirecionamento de `/__host` no mesmo helper, para o proxy do spec não poder divergir do host que ele subiu.
9. Como mantenedor, quero que o timeout de 300 s continue, para a primeira compilação do host numa worktree nova não virar falha.
10. Como mantenedor, quero que os specs continuem escrevendo `hostPort + N`, para a convenção de portas documentada no `AGENTS.md` continuar verdadeira.
11. Como mantenedor, quero que cada spec continue com acesso ao seu `home`, para os cenários que preparam estado (recentes, instalador falso, adaptador real de debug) não perderem poder de preparação.
12. Como mantenedor, quero um teste que suba um decoy na porta auxiliar e assere a falha, para a garantia não regredir em silêncio.
13. Como mantenedor, quero esse teste fora da lista do smoke noturno, para a lista curada pela #67 não crescer sem decisão.
14. Como mantenedor, quero a nota errada de "host órfão" corrigida nas notas de implementação, para ninguém caçar um fantasma que o `exec` do `cargo run` já resolve.
15. Como mantenedor, quero a decisão de **não** usar porta efêmera registrada com o motivo, para a próxima sessão não reabrir a discussão sem contexto.
16. Como mantenedor, quero que esta frente não toque em `crates/`, `src/` nem `AGENTS.md`, para o PR continuar de risco baixo e não disputar revisão do mantenedor.

## Implementation Decisions

- **Módulo novo: `tests/e2e/isolated-host.ts`.** API única: `await startIsolatedHost({ port, homePrefix, env })` devolve `{ port, base, home, useFor(page), stop() }`. `port` é o número que o spec escolheu (`hostPort + N`), `homePrefix` é o prefixo do `mkdtemp` que hoje aparece em cada spec, `env` são as variáveis que o spec precisa (`OPENRIDR_TRUST_STORE`, `OPENRIDR_ENABLE_PROBE`, `OPENRIDR_TOOLS_MUXER`, `OPENRIDR_BUNDLED_NETCOREDBG`, …). O helper sempre define `OPENRIDR_HOME` como o home temporário que ele criou.
- **Posse provada por `pid`.** O `/health` do host já devolve `pid`. O `cargo run` faz `exec` no macOS e no Linux, então o pid do filho **é** o pid do `openridr-host-http` (medido: `/health` devolveu o mesmo pid do `spawn`, e `kill()` no filho mata o host). A verificação não precisa de mudança no binário Rust.
- **Três falhas explícitas, todas nomeando a porta:** (1) o filho saiu antes de responder — mensagem com o código de saída; se um `/health` respondeu nesse meio-tempo, a mensagem inclui o pid estranho; (2) o `/health` respondeu com pid diferente do filho — mensagem com os dois pids; (3) estourou o timeout de 300 s — mensagem com a porta. Em qualquer uma, o helper mata o filho e apaga o home antes de lançar.
- **Sem retry e sem "espera a porta ficar livre".** Porta ocupada é erro de ambiente e tem que aparecer como erro, não virar uma espera que mascara a colisão.
- **O redirecionamento de `/__host` entra no helper** (`useFor(page)`): é a mesma regex e o mesmo `route.fulfill` que os sete specs copiam, com o mesmo comentário sobre poll em voo no fim do teste.
- **`stop()` mata o filho e apaga o home**, substituindo o par `server.kill()` + `fs.rmSync(home)` dos `afterAll`. Como o `cargo run` faz `exec`, o `kill` no filho mata o host — a nota de "host órfão" em `implementation-notes.md` está desatualizada e é corrigida por esta frente.
- **`playwright.config.ts` e o host compartilhado não mudam.** O helper é só para specs que sobem host próprio; o `webServer` do harness continua igual.
- **Convenção de portas intocada.** `hostPort + 1` … `hostPort + 7` continua sendo a regra, e `AGENTS.md`, `docs/SPEC-E2E-PORTS.md` e `docs/decisions.md` continuam verdadeiros. `OPENRIDR_E2E_UI_PORT`/`OPENRIDR_E2E_HOST_PORT` e a validação de `environment.ts` não mudam.
- **Sem dependência nova, sem script novo, sem mudança em `crates/`, `src/` ou `src-tauri/`.**

## Testing Decisions

- **Bom teste aqui prova comportamento observável do helper:** ele aceita o host que subiu, recusa o que não subiu, e limpa o que criou. Nada de espiar constante de módulo ou a forma da chamada de `spawn`.
- **Seam único: o módulo `tests/e2e/isolated-host.ts`.** É o único ponto por onde os sete specs obtêm host, então testar por ele cobre spawn, verificação de posse e ciclo de vida de uma vez. Nenhum seam mais baixo (uma função pura de comparação de pid) é proposto: testaria detalhe de implementação.
- **`tests/e2e/isolated-host.spec.ts` com três casos:**
  1. **Decoy:** um servidor de decoy responde `/health` com um pid que não é o do filho do helper na porta auxiliar; `startIsolatedHost` rejeita com mensagem que contém o número da porta. Falha-antes: hoje o poll aceita o decoy e o spec segue.
  2. **Caminho feliz:** o helper devolve a porta, o `/health` responde com o pid do processo que ele subiu, e o `home` existe no disco.
  3. **`stop()`:** depois de parar, a porta deixa de responder e o `home` sumiu.
- **Os sete specs migrados são a prova de não-regressão** e são rodados na verificação da frente (não viram teste novo).
- **Fora da lista do smoke noturno.** `scripts/e2e-smoke.txt` é curada pela #67; acrescentar spec lá é outra decisão.
- **Prior art:** o próprio `tests/e2e/environment.ts` (validação de porta, seed de ferramentas) para o estilo de módulo de harness, e os `beforeAll`/`afterAll` dos sete specs para o contrato que o helper precisa preservar.

## Out of Scope

- **Porta efêmera (`--port 0`).** Mataria a classe do problema em vez do sintoma, mas exige mudar `crates/host-http/src/main.rs` para imprimir a porta concedida (`server_addr()`), reescrever a convenção de 10 portas em `AGENTS.md`/`docs/SPEC-E2E-PORTS.md`/`docs/decisions.md` e tornaria o critério "porta ocupada falha nomeando a porta" impossível de demonstrar. Fica como follow-up na retro, sem issue agora.
- Consertar os specs quarentenados (#43/#67): `debugger`, `csharp-ls:150`, `testsuite`, `tests:317`, `ui-layout` em 1440 px, `find-usages`, `run-configs`, `tests`.
- Mudar o host compartilhado do harness, o proxy do Vite de teste, o `host-http` (CORS, autenticação) ou o `playwright.config.ts`.
- Rodar Playwright em PR ou mexer no workflow do smoke.
- Windows e macOS Intel.

## Further Notes

- **Por que `pid` e não porta efêmera.** O `pid` resolve a pergunta que o spec realmente faz — "o processo que respondeu é meu?" — sem tocar em produto nem em convenção, e mantém o critério de aceite demonstrável com um decoy. Porta efêmera resolveria a colisão na origem, ao custo de mudança em binário Rust, reescrita de convenção e um PR de risco alto; fica registrada como follow-up.
- **O que o `pid` não prova.** Que o host está *saudável*: um host do próprio spec que subiu e travou responde `/health` igual. A prova de saúde continua sendo o próprio teste do spec.
- **Risco do PR.** A frente toca `tests/e2e/**` e `docs/**`: `risk:low`, auto-merge quando `ci` e `security` ficarem verdes.
