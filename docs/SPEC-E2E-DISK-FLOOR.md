# SPEC — E2E recusa a rodada abaixo do piso duro de disco (issue #270)

Status: ready-for-agent
Fonte: issue GitHub #270 + grill de 2026-11-07 com o orquestrador (Q1–Q5) e o fato medido que corrige o
texto da issue (comentário na #270). Pisos herdados da #259 (`SPEC-DOCTOR-DISK-SPACE`).
Tickets: filha de #270 (criada por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

O `doctor.sh` do `verify-openridr` reporta o espaço livre no volume do `$TMPDIR` desde a #259, com `FAIL`
abaixo de 1024 MiB. Mas o doctor depende de alguém rodá-lo e obedecer: uma sessão que não roda o doctor
— ou que lê o `FAIL` e segue — começa a rodada de E2E e morre com `ENOSPC` no meio de uma cópia de
arquivo, minutos depois. Foi o que aconteceu na #217 (127 MiB livres, `globalSetup` morto no download do
csharp-ls) e de novo em 2026-11-07 (201 MB livres, ~4.600 pastas `.tmp*` no `$TMPDIR`).

O `globalSetup` do Playwright é o único ponto que toda rodada atravessa, mas **não** é o primeiro: o
`webServer` do harness é um plugin, e o Playwright executa os plugin setups antes dos global setups
(medido em 1.65, ver Further Notes). Quando o `globalSetup` começa, o host e o Vite já subiram e o
`cargo run` já compilou. O único ponto anterior a qualquer servidor é o load do `playwright.config.ts`.

## Solution

No load do `playwright.config.ts` — antes de qualquer servidor, de qualquer `cargo run` e de qualquer
teste — o harness mede o espaço livre no volume de `os.tmpdir()` e decide:

- **abaixo de 1024 MiB:** recusa a rodada, com exit não-zero e uma mensagem que traz o número medido, o
  piso e o diretório medido, no mesmo vocabulário da linha do `doctor.sh`;
- **entre 1024 e 3071 MiB:** segue, com uma linha `note` no log;
- **3072 MiB ou mais:** segue em silêncio, sem mudança de comportamento nem de tempo de setup;
- **medição sem número** (`df` ausente ou saída estranha): segue com uma linha `note`, nunca um `ok`
  silencioso e nunca uma recusa por falta de informação.

Os dois pisos têm uma fonte só: o `doctor.sh`. Um teste de `tests/unit` lê os números do script e compara
com as constantes do harness, então mexer num sem o outro reprova.

## User Stories

1. Como sessão de agente prestes a rodar E2E, quero que o harness recuse a rodada abaixo do piso duro,
   para não perder minutos até um `ENOSPC` no meio de uma cópia de arquivo.
2. Como sessão de agente, quero a recusa **antes de qualquer servidor**, para não gastar um `cargo run`
   nem portas do par escolhido numa rodada que já ia morrer.
3. Como sessão de agente, quero a mensagem com o número medido, o piso e o diretório medido, para saber
   quanto falta e qual volume está apertado (o interno, onde fica o `$TMPDIR`, e não o externo do
   worktree).
4. Como sessão de agente, quero a mensagem no mesmo vocabulário da linha do `doctor.sh`, para reconhecer
   o mesmo problema sem traduzir dois formatos.
5. Como sessão de agente com espaço entre os dois pisos, quero um aviso e não uma recusa, para poder
   rodar um spec único sabendo do risco.
6. Como sessão de agente com disco folgado, quero que nada mude — nem mensagem, nem tempo de setup — para
   o check não virar ruído.
7. Como sessão de agente numa máquina sem `df` legível, quero que a rodada siga com um aviso, para um
   instrumento quebrado não bloquear o trabalho.
8. Como mantenedor, quero que os pisos venham do `doctor.sh` e não de uma segunda cópia, para a política
   de disco não divergir entre o check que avisa e o harness que recusa.
9. Como mantenedor, quero um teste que prove a recusa sem encher um disco, para a prova caber numa
   máquina de trabalho.
10. Como mantenedor, quero o teste no CI de PR (job `typescript`), para a regra não depender de alguém
    rodar Playwright à mão.
11. Como mantenedor, quero que a medição seja somente leitura, para o harness nunca apagar nada do
    `$TMPDIR` (a limpeza é outra frente).
12. Como mantenedor, quero zero dependência nova e zero variável só-de-teste no harness, para o check ser
    só `df` e as constantes.
13. Como sessão futura, quero a cláusula no skill `verify-openridr`, para quem lê a convenção de E2E
    saber que o harness recusa a rodada abaixo do piso.

## Implementation Decisions

- **Onde a checagem roda: no load do `playwright.config.ts`, não no `globalSetup`.** O texto da issue
  pedia `globalSetup`; o fato medido mostra que ali os servidores já subiram. A checagem fica no topo do
  config, que o Playwright carrega antes de montar plugins e servidores — a mesma decisão que a #258
  tomou para portas restritas ("falha no load, de propósito, não numa função chamada pelo
  `globalSetup`"). Consequência aceita: `playwright test --list` também recusa abaixo do piso (mesma
  política, sem modo especial).
- **Um módulo novo do harness, sem efeito colateral no import.** `tests/e2e/disk-floor.ts` guarda as duas
  constantes, o parser da saída do `df` e o guarda que mede e decide. Importar o módulo não mede nada; só
  a chamada mede. É o que deixa `tests/unit` importar as funções puras sem rodar `df` nem ser afetado
  pelo disco da máquina.
- **Medição igual à do `doctor.sh`:** `df -Pk <diretório>`, coluna `Available` (1024-blocos) dividida por
  1024 dá MiB inteiro. `-P` porque o `df` do BSD muda o layout das colunas sem ele. Saída sem número
  vira "desconhecido", nunca `0` (a lição da #259: `0` é um valor válido e mentiroso).
- **Diretório medido: sempre `os.tmpdir()`.** É onde o harness põe o app home por porta da UI e o
  diretório de capturas por default. Quando `OPENRIDR_E2E_HOME` ou `OPENRIDR_SCREENSHOT_DIR` apontam
  para outro volume, o número continua sendo o do `$TMPDIR` — o override é escolha da sessão, e a
  mensagem imprime o diretório medido para o número não mentir.
- **Pisos: 1024 MiB duro, 3072 MiB de conforto**, herdados da #259 (até 4 sessões em paralelo, ~200 MiB
  de app home de E2E cada, mais caches). As constantes do harness são comparadas com o `doctor.sh` por um
  teste; o motivo fica escrito ao lado do número, como no script.
- **Três verdictos e um caso de falha de medição:** `>= 3072` silencioso; `1024..3071` uma linha `note`
  em `console.warn` e a rodada continua; `< 1024` lança erro com número, piso e diretório; medição sem
  número uma linha `note` de "unknown" e a rodada continua (fail-open, como o doctor sai 0 nesse caso).
- **Vocabulário da mensagem:** o corpo é o da linha do doctor — `disk <N> MiB free on <diretório> (under
  the <piso> MiB hard floor: ...)` — com o prefixo `E2E refused:` no erro, para o texto do Playwright
  dizer o que foi recusado.
- **Seam: o `df` resolvido pelo `PATH`.** É a fronteira de processo que o `tests/shell/doctor.test.sh` já
  usa; nenhuma variável só-de-teste entra no harness, nenhum caminho novo de configuração.
- **Somente leitura.** O check mede; não apaga, não conta `.tmp*`, não limpa o `$TMPDIR`.
- **Sem mudança de produto.** Nada em `src/`, `crates/`, `src-tauri/`, specs existentes, `data-testid`,
  `AGENTS.md` ou `GLOSSARY.md`.
- **Doc de quem roda E2E:** uma cláusula na seção Playwright do skill `verify-openridr` diz que o harness
  recusa a rodada abaixo de 1024 MiB antes de subir servidor, e que entre 1024 e 3071 ele avisa.
- **Risco do PR: `medium`.** `playwright.config.ts` é caminho que nenhuma regra da tabela de
  `scripts/ci-risk-tier.sh` conhece (default `medium`); `tests/**` e `docs/**` são `low`.

## Testing Decisions

- **O que um bom teste prova aqui:** o comportamento observável — exit code e o que a mensagem diz —, não
  como o MiB é calculado nem o formato interno das constantes.
- **Seam único: o `df` falso primeiro no `PATH`**, no molde de `tests/shell/doctor.test.sh`. O stub
  responde só à invocação que o harness deve fazer (`df -Pk <diretório medido>`) e falha em qualquer
  outra, para trocar o volume ou tirar o `-P` reprovar em vez de passar num stub que responde a tudo.
- **Teste em `tests/unit/`** (roda no job `typescript` do CI, sem harness):
  - **fonte única dos números:** o teste lê o `doctor.sh`, extrai os dois pisos das comparações `-ge` do
    bloco de disco e exige que sejam exatamente as constantes do harness. Um parser que não acha os dois
    números reprova (nada de passar vazio quando o script muda de forma);
  - **parser:** saída normal de `df -Pk` vira MiB inteiro; saída vazia, sem número ou com lixo vira
    "desconhecido" (nunca `0`);
  - **verdictos:** 4000 MiB não escreve nada; 1500 MiB escreve uma linha `note` e não lança; 900 MiB
    lança com o número, o piso e o diretório na mensagem;
  - **wiring:** um subprocesso carrega o `playwright.config.ts` com o `df` falso de 900 MiB primeiro no
    `PATH` e sai não-zero com a mensagem — é o que prova que o config chama o guarda no load, e não o
    teste da função pura sozinho. O caminho acima do piso não precisa de subprocesso.
- **A recusa real pelo CLI é evidência, não teste permanente:** `bunx playwright test --list` com o `df`
  falso de 900 MiB tem que falhar em segundos, antes de qualquer servidor, com a mensagem; e a rodada
  normal tem que continuar imprimindo o `note` (ou nada). Comando e saída vão para
  `docs/evidence/e2e-disk-floor.md`, como a #258 fez.
- **Falha-antes/passa-depois:** hoje o `playwright.config.ts` não mede nada, então o caso de 900 MiB sai
  zero e o teste novo reprova; depois, sai não-zero com a mensagem.
- **Prior art:** `tests/unit/e2eRestrictedPorts.test.ts` (subprocesso provando o wiring do módulo de
  ambiente), `tests/shell/doctor.test.sh` (`df` falso e as três severidades) e
  `tests/unit/e2eScreenshotsDir.test.ts` (função pura do harness).
- **Nada de suíte completa por reflexo:** a área é o teste unitário novo; um spec de área é reexecutado
  uma vez para provar que o harness continua subindo normalmente acima do piso.

## Out of Scope

- Limpar o `$TMPDIR`, contar ou apagar as pastas `.tmp*` órfãs (decisão do mantenedor: outra frente).
- Medir o volume do repositório, do `target/` ou do worktree externo.
- Medir o diretório de override (`OPENRIDR_E2E_HOME`, `OPENRIDR_SCREENSHOT_DIR`) em vez do `$TMPDIR`.
- Mudar o `doctor.sh`, os pisos ou a política de concorrência do orquestrador.
- `AGENTS.md` e `GLOSSARY.md`.
- Consertar os specs quarentenados (#43/#67).
- Windows e macOS Intel.

## Further Notes

- **Por que não `globalSetup` (o fato medido).** Em Playwright 1.65 o `webServer` é um plugin e
  `createGlobalSetupTasks` roda os plugin setups antes dos global setups
  (`node_modules/playwright/lib/runner/index.js:6509-6515` e `:7130`). Num projeto mínimo com `webServer`
  e `globalSetup`, o `globalSetup` fez `fetch` na URL do `webServer` e recebeu `server already up =
  true`. Um check no `globalSetup` recusaria a rodada depois de o host subir e o `cargo run` compilar —
  que é metade do custo que ele existe para evitar.
- **Por que fail-open na medição que falha.** O doctor já trata "sem número" como `note` e sai 0; o
  harness segue a mesma política. Recusar por falta de `df` trocaria um risco medido por um bloqueio
  certo, e `df` não é requisito do E2E.
- **Por que o teste lê o `doctor.sh` em vez de importar um número.** Os dois arquivos são linguagens
  diferentes; o teste é a ponte. O `doctor.sh` é a fonte porque é o que a sessão roda antes de qualquer
  coisa e o que o `SKILL.md` documenta; o harness é o consumidor.
- **O que continua sem prova.** Disco realmente cheio: a recusa é provada com `df` falso; encher o volume
  interno não é aceitável nesta máquina. Nenhuma interface do produto muda, então nenhuma superfície de
  UI é exercitada.
