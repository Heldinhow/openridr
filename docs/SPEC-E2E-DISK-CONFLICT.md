# SPEC — Conflito de disco determinístico no E2E (issue #273)

Status: ready-for-agent
Fonte: issue GitHub #273 (Heldinhow/openridr), filha de #245; grill de 2026-11-07 com o orquestrador (Q1–Q3).
Tickets: #278 (issue filha de #273).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

O teste `tests/e2e/workspace.spec.ts` "compares buffer and disk and revalidates a second disk change
before overwriting" disputa o **poll do disco** — o `setInterval` de 1,5 s do `WorkspaceView` que re-lê o
arquivo ativo. O teste escreve a segunda versão em disco e clica `conflict-overwrite` esperando um
**segundo** banner de conflito. Se o poll rodar entre a escrita e o clique, o app atualiza o conflito para a
versão nova (comportamento correto), o `overwriteDisk` usa o hash novo, a escrita passa e a segunda versão
em disco é **sobrescrita em silêncio**: o banner não nasce, ou nasce e some, e o assert falha.

Medido nesta sessão (portas 2225/5117, macOS Apple Silicon):

- sem ajuda, 20 rodadas do teste: **20 verdes**. A janela entre a escrita e o clique é de ~100 ms contra um
  tick de 1500 ms, então a corrida é rara nesta máquina;
- com **um** tick de poll forçado na janela (`waitForTimeout(1600)` entre a escrita e o clique, patch
  temporário, não commitado): **5 rodadas, 5 falhas**, todas no mesmo ponto —
  `expect(fs.readFileSync(program, "utf8")).toBe("// second disk version\n")` recebendo
  `"// buffer version\n"`. É a segunda versão em disco sobrescrita em silêncio, exatamente o que o teste
  existe para impedir;
- no runner da #245 (run 42412536089) a mesma corrida apareceu como `toBeVisible` do banner falhando.
  Qual dos dois asserts estoura depende de a escrita terminar antes ou depois da primeira checagem: o
  `click` resolve quando o evento é despachado, não quando o `file.write` volta.

Nada disso é defeito do app: o poll atualizar o conflito pendente é o desenho aprovado.

## Solution

O teste passa a **segurar** o poll em vez de disputar com ele. Antes da segunda escrita, o spec intercepta
no bridge do host (`POST /__host/call`) a chamada `file.read` do arquivo ativo e a deixa parada num gate;
espera o poll bater nessa leitura, escreve a segunda versão em disco e clica `conflict-overwrite`. A
leitura segurada nunca chega ao host, então nenhuma resposta pode carregar a segunda versão e substituir o
conflito pendente. O `expected_hash` do overwrite continua sendo o da primeira versão, o host responde
`conflict` e o segundo banner nasce do erro — determinístico, sem sorte de timing.

O app não muda: nenhuma linha de `src/`, nenhum `data-testid`, nenhuma dependência nova.

## User Stories

1. Como mantenedor, quero que o teste do conflito de disco passe sempre, para o smoke noturno não ficar vermelho por sorte de timing.
2. Como mantenedor, quero que uma falha desse teste signifique defeito no app, para nenhuma sessão gastar uma rodada investigando um flake.
3. Como mantenedor, quero que o teste continue provando que a segunda mudança em disco não é sobrescrita em silêncio, para o critério de aceite da #273 não ser trocado por uma asserção mais fraca.
4. Como mantenedor, quero que o teste prove que o poll realmente tentou re-ler e foi segurado, para o gate não poder ser removido sem o teste ficar vermelho.
5. Como mantenedor, quero que a prova seja de comportamento observável — o conteúdo em disco, o banner, o texto do editor —, para o teste não depender da estrutura interna do app.
6. Como mantenedor, quero zero mudança em código de produção, para o teste não criar um contrato novo do app.
7. Como sessão futura, quero um helper nomeado e documentado no spec, para reusar o mesmo seam em outro teste que dispute o poll.
8. Como sessão futura, quero a medição antes/depois registrada, para não reabrir a investigação do zero.
9. Como mantenedor, quero que a interceptação seja desfeita mesmo se uma asserção falhar, para o resto do teste não herdar o gate.
10. Como mantenedor, quero que o teste continue na casa dos segundos, para o smoke não crescer.
11. Como mantenedor, quero que o outro teste que prova o poll — o do buffer limpo, sem evento de foco — continue como está, para o caminho do poll não ficar sem prova nenhuma.
12. Como mantenedor, quero zero dependência nova e nenhum `data-testid` renomeado, para a frente ser só o teste.

## Implementation Decisions

- **Seam no bridge, não no app.** A leitura é segurada com `page.route("**/__host/call")`, o mesmo padrão
  que `gates.spec.ts` usa para segurar `trust.grant`. Nada de variável de ambiente nova, hook em `window`
  ou intervalo configurável em produção: o app continua com o poll de 1,5 s.
- **Segurar, não responder.** O handler espera o gate e depois chama `route.continue()`; a requisição só
  chega ao host depois do release. Responder com um corpo falso (`route.fulfill`) seria mais curto e
  mentiria sobre o disco.
- **Esperar o poll bater no gate é o auto-check do seam.** O teste espera a leitura entrar no gate antes de
  escrever a segunda versão. Sem essa espera o teste passaria mesmo com o gate removido (a janela é de
  ~100 ms em 1500 ms, ~7% por rodada); com ela, remover o gate derruba o teste sempre.
- **Release no `finally`, com o handler terminado antes do `unroute`.** Chamar `page.unroute` enquanto o
  handler ainda está parado no gate faz o `route.continue()` estourar `Route is already handled!` (medido).
  O release resolve o gate, espera o handler terminar e só então remove a rota — o mesmo desenho do helper
  de `gates.spec.ts`.
- **Escopo do gate: um caminho só.** O handler só segura `file.read` cujo `params.path` é o arquivo ativo;
  o poll de `workspace.state` (400 ms) e as escritas passam direto.
- **Nenhuma espera fixa.** Nada de `waitForTimeout` no teste: o que espera é a condição, não o relógio.
- **O caminho do poll continua provado por outro teste.** "reloads a clean buffer and flags a dirty one on
  an external write without focus" prova a re-leitura periódica (buffer limpo adota o disco, buffer sujo
  vira conflito) esperando o banner com timeout; este teste não precisa cobrir isso.
- **Risco do PR: baixo.** Só `tests/**` e `docs/**` pela tabela de `scripts/ci-risk-tier.sh`.

## Testing Decisions

- **Bom teste aqui** prova o que o usuário vê e o que está em disco: o banner, o texto comparado, o
  conteúdo do arquivo. Nada de espiar o `setInterval`, o `checkingDisk` ou o `diskEpoch`.
- **Seam único: o bridge do host** (`/__host/call`), por onde todo spec já fala com o host. Nenhum seam
  novo no app.
- **Falha-antes/passa-depois.** Com um tick de poll na janela (delay temporário de 1,6 s), o teste atual
  falha 5/5 no assert do conteúdo em disco; com o gate, o mesmo delay passa 5/5 e o teste final passa
  20/20 sem delay.
- **Prior art:** `delayNextTrustGrant`/`rejectNextTrustGrant` em `tests/e2e/gates.spec.ts` (gate +
  `finished` + `unroute`) e o teste irmão do poll no mesmo spec.
- **Regressão da área:** o spec inteiro roda uma vez no fim, além dos repeats do teste alvo.
- **Sem app real:** nada aqui é específico do shell (menu nativo, atalho do sistema, seletor nativo,
  árvore de acessibilidade do WebKit, updater, DMG).

## Out of Scope

- `run.spec.ts:352` / #251: a build falhou antes; é outra família, como a nota da issue diz.
- Qualquer mudança em `src/` — o poll, o `checkDisk`, o `overwriteDisk` e o `diskEpoch` ficam como estão.
- Quarentenar o spec ou tirá-lo do smoke.
- Os outros specs do smoke.
- Windows e macOS Intel.

## Further Notes

- **Por que não "afirmar o estado final".** A issue oferece essa saída; ela enfraquece o teste: com o poll
  atualizando o conflito, o overwrite passa e o estado final (disco = buffer) fica indistinguível do
  caminho correto. O que o teste precisa provar é a revalidação do `expected_hash`, e para isso o conflito
  tem de estar intacto no clique.
- **Por que não forçar a re-leitura com `dispatchWindowFocus`.** Seria determinístico, mas prova o
  contrário: o conflito passa a mostrar a versão nova e o overwrite então grava — some a prova de que a
  segunda mudança não é sobrescrita em silêncio.
- **O que continua sem prova.** O runner do GitHub: a corrida foi medida localmente com um tick forçado,
  não no CI. O `workflow_dispatch` do smoke não é disparado por esta sessão (o grupo de concorrência está
  em uso pela #251).
