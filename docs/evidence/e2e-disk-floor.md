# Evidência — o config do Playwright recusa a rodada abaixo do piso duro de disco (issue #270, ticket #287)

Data: 2026-11-07. Ambiente: macOS Apple Silicon, `~/.dotnet` no `PATH`, volume do `$TMPDIR` com ~24.000
MiB livres. Par de portas: 2525 (UI) / 5417 (host) na implementação; 2525 (UI) / 5427 (host) na
verificação da sessão, porque a 5417 estava ocupada por outra sessão.

Spec: [SPEC-E2E-DISK-FLOOR](../SPEC-E2E-DISK-FLOOR.md). Ticket: #287 (filho de #270).

## 1. A recusa acontece no load, antes de qualquer servidor (critério da issue)

`playwright test --list` carrega a config e nada mais: não sobe host, não compila `cargo run`, não sobe
Vite, não abre navegador. O `df` falso primeiro no `PATH` responde 900 MiB e só aceita
`df -Pk <diretório medido>` (qualquer outra invocação sai 1).

```sh
# stub: responde só `df -Pk $FAKE_DF_EXPECT`, Available = $FAKE_DF_MIB * 1024
env PATH="$scratch:$PATH" FAKE_DF_MIB=900 FAKE_DF_EXPECT="$TMPDIR" \
  OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5417 bunx playwright test --list
# exit=1 em 2 s
# Error: E2E refused: disk 900 MiB free on /var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T
#   (under the 1024 MiB hard floor: up to 4 parallel sessions, ~200 MiB E2E home each + caches)
#     at guardDiskFloor (tests/e2e/disk-floor.ts:61:41)
#     at playwright.config.ts:7:1
#     at loadUserConfig (node_modules/playwright/lib/common/index.js:1283:46)
```

O stack mostra o caminho: `playwright.config.ts` linha 7 (topo do arquivo, antes do `defineConfig`) →
`guardDiskFloor`. Nenhum servidor foi montado.

## 2. Entre os pisos: uma linha `note` e a rodada continua

```sh
env PATH="$scratch:$PATH" FAKE_DF_MIB=1500 FAKE_DF_EXPECT="$TMPDIR" \
  OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5417 bunx playwright test --list
# exit=0
# stderr: note  disk 1500 MiB free on /var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T
#   (under the 3072 MiB floor: up to 4 parallel sessions, ~200 MiB E2E home each + caches; run one spec at a time)
# stdout: Listing tests: ... / Total: 312 tests in 64 files
```

## 3. Acima do piso: nada muda

```sh
OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5417 bunx playwright test --list
# exit=0 em 7 s, sem nenhuma linha de disco no stderr
# Total: 312 tests in 64 files
```

## 4. O harness continua subindo normalmente acima do piso

```sh
OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5417 bunx playwright test tests/e2e/environment.spec.ts
# 3 passed (9.4s), exit 0, sem nenhuma linha de disco (o volume real tem ~24.000 MiB)
```

O `webServer` subiu o host (`cargo run` já compilado) e o Vite, e os três testes do harness passaram:
validação de porta + proxy `/__host`, app home derivado da porta e o host usando o home isolado. É a
prova de que a checagem no load não atrapalha a rodada normal.

## 5. Teste unitário (roda no CI de PR, job `typescript`)

```sh
bun test tests/unit
# 132 pass, 0 fail, 410 expect() calls, 18 files
```

O arquivo novo (`tests/unit/e2eDiskFloor.test.ts`) tem 9 testes:

- **fonte única:** lê o `doctor.sh` e exige que as duas comparações `-ge` do bloco de disco sejam
  exatamente `3072` e `1024` (as constantes do harness); um parser que não acha os números reprova com
  `[] != [3072, 1024]`, nunca passa vazio;
- **parser:** a coluna `Available` de `df -Pk` vira MiB inteiro (`24601928` → `24025`); saída vazia, sem
  número ou com lixo vira `undefined`, nunca `0`;
- **verdictos:** 4000 MiB não escreve nada; 1500 MiB é `note` com o número e o diretório; 900 MiB é
  `refuse` com `E2E refused:` + número + piso + diretório; medição sem número é `note`, nunca recusa;
- **wiring:** dois subprocessos carregam o `playwright.config.ts` de verdade (`bun -e 'await
  import("./playwright.config.ts")'`) com o `df` falso primeiro no `PATH`: 900 MiB sai não-zero com a
  mensagem, 1500 MiB sai zero com a linha `note` no stderr.

Falha-antes medido: com o módulo ainda inexistente, `bun test` deu `0 pass / 1 fail / 1 error`; com o
módulo mas sem a chamada no config, `7 pass / 2 fail` (os dois casos de subprocesso). Depois da chamada,
`9 pass / 0 fail`.

O stub também foi exercitado contra uma mudança de flags: trocar `df -Pk` por `df -k` no harness faz o
stub recusar a invocação, a medição vira "unknown" e os dois casos de subprocesso reprovam — o teste não
passa num stub que responde a qualquer coisa.

## 6. Verificação da sessão, depois do merge (par 2525/5427)

Rodei de novo, na integration branch, com o par conferido livre (`lsof` nas 2525 e 5427, mais a faixa
derivada 5428–5434) e o `doctor.sh` verde (disco 21587 MiB livres).

```sh
# a) recusa, rodada real (não --list), com `cargo` e `bun` armadilha no PATH que criariam um sentinela
env PATH="$scratch:$PATH" FAKE_DF_MIB=900 FAKE_DF_EXPECT="$TMPDIR" \
  OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5427 \
  bunx playwright test tests/e2e/environment.spec.ts
# exit=1
# Error: E2E refused: disk 900 MiB free on /var/folders/n4/.../T
#   (under the 1024 MiB hard floor: up to 4 parallel sessions, ~200 MiB E2E home each + caches)
#   at guardDiskFloor (tests/e2e/disk-floor.ts:61:41)
#   at playwright.config.ts:7:1
# nenhum sentinela CARGO-SPAWNED/BUN-SPAWNED: nenhum servidor foi spawnado

# b) entre os pisos
env PATH="$scratch:$PATH" FAKE_DF_MIB=1500 ... bunx playwright test --list
# exit=0; stderr: note  disk 1500 MiB free on /var/folders/n4/.../T (under the 3072 MiB floor: ...)
# Total: 312 tests in 64 files

# c) acima do piso, df real
OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5427 bunx playwright test --list
# exit=0; nenhuma linha de disco

# d) o harness sobe normal acima do piso
OPENRIDR_E2E_UI_PORT=2525 OPENRIDR_E2E_HOST_PORT=5427 bunx playwright test tests/e2e/environment.spec.ts
# 3 passed (1.4m) — host (cargo run) e Vite subiram, app home isolado conferido

bun test tests/unit
# 132 pass, 0 fail, 410 expect() calls, 18 files
```

O caso (a) é mais forte que o `--list` do item 1: é uma rodada de verdade, e os dois stubs de servidor
provam que a recusa acontece **antes** de qualquer `cargo run` ou `bun run dev` — com o guarda no
`globalSetup` os dois sentinelas existiriam.

## 7. Achados do code review (corrigidos)

- **Médio — o parser aceitava linha que não é `df`.** Qualquer segunda linha com uma quarta palavra
  numérica virava MiB: `garbage\nfoo bar baz 0` dava `0` e **recusava** a rodada (contrariando o
  fail-open), e um número de 400 dígitos virava `Infinity` e dava `ok` silencioso. Medido no código
  anterior:

  ```sh
  bun -e 'const m = await import("/tmp/old-disk-floor.ts"); …'
  # velho, linha lixo: 0
  # velho, número gigante: Infinity
  # velho, veredito do lixo: refuse
  ```

  O parser agora exige a forma de uma linha de dados de `df -Pk` (seis campos, `Available` só dígitos,
  `Capacity` terminando em `%`) e um inteiro seguro; fora disso é desconhecido, nunca `0` nem `ok`. Dois
  testes novos cobrem os dois casos (11 testes no arquivo).
- **Baixo — o stub do `df` aceitava argumento extra.** Agora exige `$#` igual a 2: `df -Pk <dir> <outro>`
  reprova, como o ticket pede.

## Não verificado

- **Disco realmente cheio:** a recusa é provada com `df` falso; encher o volume interno não é aceitável
  nesta máquina (foi o motivo da #217).
- **`df` ausente de verdade:** o caminho "medição sem número" é provado pela função pura e pelo stub que
  recusa a invocação; tirar o `df` do `PATH` de uma sessão real não foi feito.
- **`OPENRIDR_E2E_HOME`/`OPENRIDR_SCREENSHOT_DIR` noutro volume:** por decisão, a medição continua sendo a
  do `$TMPDIR` e a mensagem imprime o diretório medido; o override não foi exercitado.
- **Suíte E2E completa e smoke noturno:** não rodados; a área é o teste unitário e um spec de harness.
- **App Tauri real, Windows e macOS Intel:** nada aqui é superfície nativa.
