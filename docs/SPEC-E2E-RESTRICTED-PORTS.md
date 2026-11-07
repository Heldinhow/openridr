# SPEC — E2E recusa porta restrita do Chrome (issue #258)

Status: ready-for-agent
Fonte: issue GitHub #258 (Heldinhow/openridr) + grill de 2026-11-07 com o orquestrador (Q1–Q5).
Tickets: issues filhas de #258 (criadas por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

A sessão escolhe um par de portas para o E2E, o harness sobe normalmente — o host responde `/health` e o
Vite escuta — e o primeiro `page.goto` morre com `net::ERR_UNSAFE_PORT`. Medido na sessão da #217
(2026-11-07) com o par 1720/4617. O validador de porta confere só que o valor é um inteiro de 1 a 65535;
ele não conhece a lista de portas que o **Chromium recusa por política**, e o Playwright usa Chrome por
default. O sintoma parece bug do app, e a sessão perde a rodada inteira até alguém reconhecer a mensagem.

O mesmo vale para as portas derivadas: sete specs sobem host próprio em `hostPort + 1` até `+ 7`, e o
navegador alcança essas portas direto. Um `hostPort` cuja faixa derivada toca uma porta restrita (1716 →
1719) quebra exatamente do mesmo jeito, só que no meio do run, com o mesmo erro ilegível.

## Solution

O validador de porta do E2E recusa porta restrita **na hora**, com uma mensagem que nomeia a porta e diz
que o Chrome a bloqueia. Vale para as duas variáveis (`OPENRIDR_E2E_UI_PORT` e `OPENRIDR_E2E_HOST_PORT`) e
para a faixa derivada `hostPort + 1` até `+ 7`. Como a validação roda no load do módulo de ambiente, que o
`playwright.config.ts` importa antes de subir qualquer servidor, o run morre no primeiro segundo com um
erro que se explica sozinho — não no `page.goto`.

A lista é a `kRestrictedPorts` do Chromium (`net/base/port_util.cc`), copiada inteira. Na faixa que o E2E
usa, as portas restritas são **1719, 1720 e 1723**; 1721 e 1722 **não** estão na lista do navegador e
continuam aceitas. (Correção do texto da issue #258, que fala em "1719–1723": a faixa inteira não é
restrita.)

## User Stories

1. Como sessão de agente, quero que o harness recuse uma porta restrita do Chrome antes de subir o host e o
   Vite, para não descobrir no `page.goto` que perdi a rodada.
2. Como sessão de agente, quero que a mensagem de erro nomeie a porta e diga que o Chrome a bloqueia, para
   reconhecer o problema sem investigar `net::ERR_UNSAFE_PORT`.
3. Como sessão de agente, quero a mesma recusa na porta da UI e na do host, para não precisar lembrar qual
   das duas o navegador acessa direto (as duas: o host vai pelo proxy `/__host` do Vite).
4. Como sessão de agente, quero que a recusa cubra a faixa derivada `hostPort + 1` até `+ 7`, para um par
   aparentemente livre não quebrar um spec que sobe host próprio no meio do run.
5. Como sessão de agente, quero que a mensagem da faixa derivada diga **qual** porta derivada é a culpada e
   qual é o par escolhido, para eu corrigir o par em vez de adivinhar.
6. Como sessão de agente, quero que portas que o Chrome aceita continuem aceitas (1721, 1722, 1725), para a
   validação não inventar restrição que o navegador não tem.
7. Como mantenedor, quero que a validação existente (não-inteiro, vazio, `0`, `65536`, negativo, par igual)
   continue igual, para esta frente não reabrir comportamento já coberto.
8. Como mantenedor, quero a lista copiada do Chromium com a fonte e a data no comentário, para a próxima
   sessão saber de onde ela veio e quando foi verificada.
9. Como mantenedor, quero um teste que rode no CI de PR (job `typescript`), para a regra não depender de
   alguém rodar Playwright.
10. Como mantenedor, quero prova de que **as duas** variáveis passam pela recusa, para o teste não provar
    só a função pura e deixar o wiring solto.
11. Como mantenedor, quero a evidência de que o erro aparece no começo do run (`playwright --list` falhando
    em segundos), para o critério "não no `page.goto`" ter prova executada, não promessa.
12. Como sessão futura, quero a cláusula no skill `verify-openridr`, para quem escolhe o par de portas ler a
    regra no mesmo lugar em que lê a convenção de portas.
13. Como mantenedor, quero zero dependência nova, zero script novo e nenhum `data-testid` tocado, para esta
    frente ser só o validador e seus testes.

## Implementation Decisions

- **Um validador só, o que já existe.** A checagem entra na função `port()` do módulo de ambiente do E2E,
  depois da validação numérica atual; ela continua sendo o único caminho por onde as portas do harness
  passam. Nenhum módulo novo, nenhum helper paralelo.
- **Lista exata, não faixa.** A constante é a `kRestrictedPorts` do Chromium copiada inteira (inclui `0`,
  que a validação numérica já recusa), com comentário apontando `net/base/port_util.cc` e a data da cópia
  (2026-11-07). Bloquear a faixa 1719–1723 seria mais curto e recusaria porta que o Chrome aceita — a
  mensagem passaria a mentir, e o teste "1721/1722 continuam aceitos" existe justamente para pinar isso.
- **Mensagem mantém o prefixo.** O erro continua começando com `Invalid E2E port: <valor>` (grep-ável, e o
  spec existente já casa com esse texto) e acrescenta o motivo e a saída: a porta nomeada, que o Chrome a
  bloqueia, e que é preciso escolher outra.
- **Faixa derivada entra.** A porta do host reserva `hostPort + 1` até `+ 7` para os specs que sobem host
  próprio; se qualquer uma delas for restrita, o carregamento falha e a mensagem nomeia a porta derivada
  culpada e o par escolhido. É a mesma classe de falha e custa poucas linhas.
- **Falha no load, de propósito.** A checagem fica no nível do módulo (onde as constantes de porta já são
  calculadas), não numa função chamada pelo `globalSetup`: assim `playwright.config.ts` e o config do Vite
  de teste falham antes de qualquer processo subir.
- **Sem mudança de produto.** Nada em `host-http`, CORS, proxy do Vite de teste, specs existentes, `AGENTS.md`
  ou `data-testid`. `AGENTS.md` fica intocado (é `risk:high` e a mudança não pede).
- **Doc de quem escolhe porta.** Uma cláusula no parágrafo de portas do skill `verify-openridr` diz que porta
  restrita do Chrome é recusada na hora; a convenção de portas continua a mesma.
- **Risco do PR: baixo.** Os caminhos tocados (`tests/**`, `*.md`, `docs/**`) caem em `risk:low` pela tabela
  de `scripts/ci-risk-tier.sh`.

## Testing Decisions

- **Bom teste aqui prova comportamento observável do validador**: qual valor é aceito, qual é recusado e o
  que a mensagem diz. Nada de espiar o formato da constante nem a ordem das checagens.
- **Seam único: o módulo de ambiente do E2E**, que já é por onde config, `globalSetup` e specs leem portas.
  Nenhum seam novo.
- **Teste unitário em `tests/unit/`** (roda no job `typescript` do CI, sem harness, milissegundos):
  - a função recusa 1719, 1720 e 1723 com mensagem que nomeia a porta e cita o Chrome;
  - aceita 1721, 1722 e 1725 (portas que o Chrome aceita);
  - a validação existente não muda: vazio, não-inteiro, `0`, `65536`, negativo e o par igual continuam
    falhando com `Invalid E2E port`;
  - um subprocesso por variável (`OPENRIDR_E2E_UI_PORT=1720` e `OPENRIDR_E2E_HOST_PORT=1720`), importando o
    módulo de ambiente, sai com erro e a mensagem — é o que prova o wiring das **duas** variáveis;
  - `hostPort` cuja faixa derivada toca porta restrita é recusado, e a mensagem nomeia a derivada.
- **Prior art:** o teste de função pura do diretório de capturas (`tests/unit/e2eScreenshotsDir.test.ts`) e
  as asserções de porta que já existem no spec de ambiente do E2E (Playwright, que não roda em PR — por isso
  o teste novo não mora lá).
- **O "erro no começo do run" é evidência, não teste permanente:** `OPENRIDR_E2E_UI_PORT=1720` com
  `playwright test --list` tem que falhar em segundos, antes de qualquer servidor, com a mensagem. O comando
  e a saída entram em `docs/evidence/e2e-restricted-ports.md`.
- **Falha-antes/passa-depois:** o teste novo falha no código atual (1720 é aceito hoje) e passa depois.
- **Nada de suíte completa por reflexo:** a área é o teste unitário novo; o spec de ambiente do E2E é
  reexecutado uma vez para provar que nada quebrou ali.

## Out of Scope

- Helper/script que escolhe porta livre, porta aleatória, porta `0`.
- Mudar `host-http`, o CORS fixo em 1420 ou o proxy `/__host` do Vite de teste.
- Consertar os specs quarentenados (#43/#67).
- `AGENTS.md` (o texto de portas de lá continua correto para o que ele diz).
- Windows e macOS Intel.

## Further Notes

- **Por que não confiar na mensagem do Chrome.** O `ERR_UNSAFE_PORT` só aparece no `page.goto`, depois de o
  host e o Vite já estarem de pé, e o texto não diz qual das variáveis errou. Recusar no load troca um erro
  de rede por um erro de configuração.
- **Por que a lista inteira e não só a faixa do E2E.** A lista cabe numa constante e a faixa do E2E muda por
  sessão (15xx/44xx, 20xx, etc.); copiar tudo custa o mesmo que copiar três portas e não deixa a próxima
  faixa descoberta.
- **O que continua sem prova.** Nenhuma porta restrita é exercitada de fato contra o Chrome nesta frente (o
  teste prova a recusa, e a evidência prova a falha precoce); a suíte E2E completa e o app Tauri real não
  são tocados, porque nada aqui é superfície de UI.
