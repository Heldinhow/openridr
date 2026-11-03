# Verificação formal — proposta e prova de conceito

Escopo: **3 regras críticas** do host que são máquinas de estado com invariantes claros. Nada pago. Nenhuma dependência nova de runtime. Código de produto não foi tocado; só foi adicionado um teste.

Status em 2026-11-03: **1 PoC executada** (candidato 1). Os candidatos 2 e 3 são só proposta. TLA+ **não foi executado**: não há Java nem TLC nesta máquina.

## Candidatos (em ordem de valor)

### 1. Ciclo de vida do trust — PoC feita

Onde: `crates/host/src/lib.rs` (`grant_locked`, `revoke_locked`, `set_trust`, `apply_trust`, `require_trust`) e `crates/host/src/trusted.rs` (`trusted.json`).

Estado: `workspace ∈ {nenhum, aberto(chave)}`, `mode ∈ {restricted, trusted}`, decisão armazenada por chave `∈ {∅, trusted, restricted}`.
Transições: `open`, `grant`, `revoke`, `close`, `restart` (novo `Host`, mesmo `OPENRIDR_HOME`).

Invariantes provados pelo teste (`I1`–`I6`):

| # | Invariante |
|---|---|
| I1 | a flag `trusted` do host == o modelo; `trusted` exige workspace aberto |
| I2 | `mode` e `decision` do workspace aberto == modelo |
| I3 | o gate (`terminal.resize`) devolve `trust_required` **se e somente se** o workspace não é confiável |
| I4 | `trust.get` por caminho == decisão armazenada |
| I5 | `trusted.json` em disco == decisão armazenada do modelo |
| I6 | depois de reiniciar, o workspace reabre confiável **se e somente se** a decisão armazenada é `trusted` |

Por que vale: trust é a fronteira de segurança (executar `dotnet`, LSP, debug). Um erro aqui é perda de contenção, não bug visual.

### 2. Registro selado (restore, testes, debug) — proposta, bom para TLA+

Onde: `crates/host/src/restore.rs`, `test_run.rs`, `debugger.rs`, `dap.rs`. Todos usam `state` + `sealed` + `workspace_id`/`context_id`.

Máquina: `idle → running → {ok, failed, cancelled}`; `seal()` (troca de workspace/contexto) é um evento externo concorrente com `complete()`, `observe()`, `append_line()` e `note_process()` vindos de threads do processo filho.

Invariantes a modelar:

- **S1** `sealed` é absorvente: nunca volta a `false` dentro do mesmo registro.
- **S2** depois de `sealed`, `append_line`, `note_process`, `observe`, `complete`, `fail` não alteram o registro; só `seal` fixa o resultado observado.
- **S3** um estado terminal (`ok/failed/cancelled`) nunca volta a `running`.
- **S4** uma saída do processo antigo nunca é atribuída a um `context_id` mais novo.

Por que vale: é concorrência (leitores de pipe, registry de processos, troca de workspace). Testes de exemplo só cobrem entrelaçamentos escolhidos à mão; um modelo explora todos. Candidato natural a TLA+ com 3 processos (`pump`, `complete`, `seal`).

### 3. Token de contexto (resultado obsoleto) — proposta

Onde: `accept_locked` e `context_seq` em `crates/host/src/lib.rs` (~linhas 6120 e 1022/1091/1185/1235/6358).

Regra: um resultado assíncrono só é aplicado se `workspace.id == token.workspace_id && context_seq == token.context_id && trusted`; senão `stale_context`.

Invariantes:

- **C1** `context_seq` só cresce.
- **C2** todo evento que muda workspace, trust ou SDK incrementa `context_seq` (vale conferir os 6 pontos de incremento, e achar um caminho que mude contexto sem incrementar).
- **C3** um resultado aceito foi emitido sob o contexto vigente no momento da aplicação.

Por que vale: C2 é uma propriedade de cobertura (“nenhum caminho esquece de incrementar”). Isso pede busca exaustiva de transições, não teste de exemplo. Pode ser um teste de propriedade em Rust no mesmo estilo da PoC.

### Não vale modelar: pedido de abrir (`PendingOpen`)

`src-tauri/src/main.rs` + `src/App.tsx`: “o último pedido vence”. Uma variável, uma regra. Teste de unidade (`cargo test --manifest-path src-tauri/Cargo.toml`, já existe) e o E2E `open-external.spec.ts` cobrem; modelo formal custaria mais que a regra.

## Prova de conceito

Arquivo: `crates/host/tests/trust_model.rs`. Teste de propriedade guiado por modelo, sem dependência nova (gerador LCG com seed fixa; não há `proptest` nem `fast-check` no repo).

- 60 passeios aleatórios × 14 passos sobre `open / grant / revoke / close / restart`, comparando o `Host` real com um modelo de referência de ~30 linhas, nos invariantes I1–I6.
- Comando: `cargo test -p openridr-host --test trust_model` (~1,4 s).
- Resultado: passa.
- **Teste de mutação**: fiz o `Revoke` gravar `Some(true)` (revoke que mantém a decisão `trusted`). O teste falhou. Mutação revertida; `git diff` está vazio, ou seja, nenhum arquivo rastreado (código de produto) mudou.

Limites honestos:

- Não é prova: é busca aleatória limitada (840 transições por execução, seed fixa). Não cobre concorrência.
- O modelo é escrito à mão a partir da leitura do código: se eu entendi errado a regra, o modelo e o código podem concordar no erro. A fonte da regra continua sendo `docs/SPEC-WELCOME-TRUST.md`.
- Chave de trust = caminho resolvido (para pasta com `.csproj`, o do `.csproj`), canonicalizado (`/private/tmp/...` no macOS). O teste usa os caminhos canônicos.

## Proposta de TLA+ (candidato 2) — não executada

Esboço, para quando houver Java + `tla2tools.jar` (gratuito):

```tla
VARIABLES state, sealed, log, ctx
Seal      == state = "running" /\ sealed' = TRUE /\ UNCHANGED <<state, log, ctx>>
Append    == ~sealed /\ log' = Append(log, "line") /\ UNCHANGED <<state, sealed, ctx>>
Complete  == state = "running" /\ ~sealed /\ state' \in {"ok","failed","cancelled"} /\ UNCHANGED <<sealed, log, ctx>>
Next      == Seal \/ Append \/ Complete \/ NewContext
Inv       == (sealed => [](log = log))   \* após selar, o log não muda
```

Passo seguinte: escrever o `.tla`, rodar TLC com 3–4 passos de `Append`, e comparar com o `restore.rs` real linha a linha. Custo estimado: meio dia. Retorno: confiança no S1–S4, que hoje só têm testes de exemplo.

Alternativa sem Java: statechart em Mermaid + o mesmo tipo de teste de propriedade da PoC, com threads (`loom` seria dependência de dev nova — precisa de decisão registrada).

## Como isso entra no fluxo

O skill `.agents/skills/verify-openridr` aponta para este arquivo. Ao mudar trust, o teste `trust_model` roda como parte da verificação. Qualquer outra mudança nos candidatos 2–3 deve primeiro reler as invariantes acima.
