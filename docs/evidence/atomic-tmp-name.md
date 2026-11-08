# Evidência — `.tmp` do `write_json_atomic` colide entre escritas concorrentes (#294)

Issue: #294. Ambiente: macOS ARM64, worktree isolada. Data: 2026-11-07.

## Causa raiz

`crates/host/src/fsutil.rs::write_json_atomic` nomeava o temporário com
`.<arquivo>-<pid>-<SystemTime::now().as_nanos()>.tmp`. O relógio deste macOS tem resolução grossa
(a issue mediu **15.506 de 16.000** timestamps repetidos com 8 threads), então duas escritas
concorrentes no mesmo destino escolhiam o mesmo `.tmp`: uma renomeava primeiro e a outra falhava em
`fs::rename` com `No such file or directory`. Pior, o `fs::write` de uma podia truncar o arquivo que a
outra estava prestes a renomear, deixando o destino com JSON parcial — `trusted.json` lido como
`trusted_unreadable`.

## Conserto

O nome do temporário passou a vir de um contador atômico do processo (`static TEMP_SEQUENCE:
AtomicU64`, `fetch_add(1, Ordering::Relaxed)`) somado ao `pid`. O pid separa processos, o contador
separa escritas dentro do processo; nenhum dos dois depende do relógio. `SystemTime`/`UNIX_EPOCH`
saíram do arquivo. Nenhuma dependência nova, nenhuma mudança no formato ou no local do destino, nenhum
contrato do host alterado (`write_json_atomic` continua `pub(crate)` com a mesma assinatura).

## Falha-antes medida

Teste novo `fsutil::tests::concurrent_writes_to_one_path_all_succeed_and_leave_valid_json`: 8 threads ×
200 escritas no mesmo `trusted.json`, cada escrita tem de voltar `Ok` e o arquivo é relido logo depois:
tem de parsear **e** ser o seed ou uma escrita completa de alguma thread (`thread` e `iteration`
inteiros) — JSON válido porém vazio não passa.

- Com o `fsutil.rs` de `c35f60b` (relógio): **FAILED em 0,07 s**, 5 threads paniquearam já na primeira
  iteração com `operation_failed: Could not replace …/trusted.json: No such file or directory (os
  error 2)`.
- Com o conserto: **1 passed** em 0,25–0,30 s, 5 execuções seguidas.

## Bateria

- `cargo test -p openridr-host` **completo** (todos os binários): **629 passed, 0 failed**, 70 ignored —
  `lib` 471, `bundled_analysis` 11, `corporate_proxy` 2, `debugger` 15, `host` 111, `test_discovery` 11,
  `test_run` 7, `trust_model` 1.
- `cargo test -p openridr-host --lib -- trusted` **20 rodadas** (critério da issue) → 4 passed em todas
  as 20, 0 failed.
- `cargo fmt --all -- --check` limpo; `cargo clippy --workspace --all-targets -- -D warnings` limpo.

## Falha de ambiente observada (não é do conserto)

O primeiro `cargo test -p openridr-host` completo, com outros workers rodando na mesma máquina, teve
`110 passed; 1 failed` em `initialize_failure_surfaces_the_sdk_message_and_kills_the_process` (o leaf
isolado não preservou a mensagem). Isolado: **1 passed** em 3,55 s; o `--test host` completo seguinte:
**111 passed**; a bateria completa no fim da sessão: **629 passed, 0 failed**. É a **mesma hipótese**
registrada na linha 29 de [`test-stability.md`](test-stability.md) (carga; causa **não determinada**),
que agora conta a segunda observação — registrado, não é passe.

## Não verificado

- A corrida em produção (app real com dois writers no mesmo `trusted.json`): o caminho é o mesmo, mas a
  prova é o teste de threads do host.
- Windows e macOS Intel.
- E2E: nenhum spec exercita escrita concorrente de estado do host; a mudança é de host puro, sem
  superfície de UI.
