# E2E: a captura não escreve no repositório por default (issue #217)

Medição em macOS Apple Silicon (arm64), 2026-11-07, Chrome real (`channel: "chrome"`), SDK 8.0.408 das
fixtures em `~/.dotnet`, `workers: 1`. Par de portas desta sessão: **1725 (UI) / 4617 (host)** — ver
"Por que não 1720" no fim.

## O que mudou

O default do diretório das capturas saiu do repositório: agora é
`<tmpdir>/openridr-e2e-screenshots-<porta da UI>`, derivado da porta como o E2E app home da #210.
`OPENRIDR_SCREENSHOT_DIR` continua vencendo, e `docs/evidence/screenshots` só recebe captura quando a
variável aponta para lá de propósito. O caminho é calculado num lugar só (o módulo de ambiente do
E2E) e todo spec que grava captura lê de lá — inclusive os dois que gravavam num literal e ignoravam
a variável.

## Estado do diretório versionado antes da mudança

`docs/evidence/screenshots/` está no `.gitignore`, mas **86 PNGs estão versionados** (o `.gitignore`
não vale para arquivo já rastreado). O estrago do default antigo vinha em duas formas:

| Spec | Nome gravado | Efeito no repositório |
|---|---|---|
| `csharp-ls.spec.ts` | `05-csharp-ls.png`, `06-problems*.png`, `08-*`, `09-*`, `10-*` | **reescreve arquivo versionado** — aparece em `git status` |
| `find-usages.spec.ts`, `zy-gutter.spec.ts` | `d05-find-usages.png`, `d09-gutter-test.png` | idem (e ignoravam a variável) |
| `build.spec.ts` | `11-*.png` | **cria arquivo não rastreado dentro do repo** — `git status` não mostra (ignorado), mas `git worktree remove` recusa a worktree |

A segunda linha é a que a #210 registrou ("`git worktree remove` refusing to delete a throwaway
checkout"): worktree com arquivo não rastreado não é removível sem `--force`.

## Prova

Todas as execuções usaram o mesmo comando base:

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en
OPENRIDR_E2E_UI_PORT=1725 OPENRIDR_E2E_HOST_PORT=4617 bunx playwright test tests/e2e/build.spec.ts
```

| # | Cenário | Resultado | `git status --porcelain` | Capturas |
|---|---|---|---|---|
| A | sem `OPENRIDR_SCREENSHOT_DIR` | **2 passed** (1,3 min), exit 0 | **vazio** antes e depois | 5 PNGs em `<tmpdir>/openridr-e2e-screenshots-1725` |
| B | sem a variável, run interrompido no meio (`--global-timeout=25000`) | **exit 1** — `1 did not run`, `1 passed`, `Timed out waiting 25s for the test suite to run` | **vazio** depois | 4 PNGs já gravados em `<tmpdir>/openridr-e2e-screenshots-1725` |
| C1 | `OPENRIDR_SCREENSHOT_DIR=/tmp/217-shots-explicit` | **2 passed** (34,2 s), exit 0 | **vazio** | 5 PNGs em `/tmp/217-shots-explicit`; o default da porta não recebeu nada |
| C2 | `OPENRIDR_SCREENSHOT_DIR=docs/evidence/screenshots` (o caminho versionado, agora só de propósito) | **2 passed** (29,3 s), exit 0 | **vazio** (os nomes `11-*` não são versionados) | 5 PNGs criados dentro do repo, com mtime do run |

B é o critério de aceite da issue: uma execução que **termina não-zero** depois de já ter gravado
captura deixa o `git status` limpo. A foi o mesmo spec passando. C1 e C2 são o outro critério: a
variável setada escreve exatamente onde aponta, inclusive no caminho versionado.

Depois de C2 os 5 PNGs criados em `docs/evidence/screenshots/` foram apagados à mão (eram não
rastreados e ignorados); `git status` continuou vazio e nenhum PNG versionado foi tocado em nenhuma
das quatro execuções.

## Guarda no CI

`tests/unit/e2eScreenshotsDir.test.ts` (novo, roda em `bun run test:unit`, que é o que o CI executa):

- **vermelho na árvore de antes**: 15 specs nomeavam o caminho versionado (13 com o default, 2 com
  literal);
- **verde depois**: 116 testes de unidade passam, 0 falham;
- varre `tests/e2e/` **recursivamente** e compara só as letras do arquivo, então pega tanto
  `"docs/evidence/screenshots"` quanto `path.join("docs", "evidence", "screenshots")`;
- pina o default fora do repositório, a chave por porta e o override (absoluto e relativo).

## Decisões registradas nesta medição

- **`outputDir`/`playwright-report` ficam onde estão.** `test-results/` e `playwright-report/` já estão
  no `.gitignore` e não têm arquivo versionado: nenhuma das quatro execuções sujou `git status` por
  causa deles. Mover para o temporário não resolveria problema nenhum.
- **`OPENRIDR_EVIDENCE_LOG_DIR` fica como está.** O default (`docs/evidence/logs`) é ignorado e não tem
  arquivo versionado — fora do escopo da #217.

## Por que não 1720

O par reservado para esta sessão era 1720/4617. O host e o Vite subiram, mas o Chrome recusou a
navegação: `Error: page.goto: net::ERR_UNSAFE_PORT at http://127.0.0.1:1720/`. 1719–1723 estão na
lista de portas proibidas do navegador (H.323/PPTP), então nenhuma delas serve para a UI do E2E. O par
usado foi **1725/4617**, com o OK do orquestrador. A validação do harness (`port()`) só confere
1–65535: ela não conhece a lista do Chrome, e quem escolher uma dessas portas perde a rodada inteira.

## Não verificado

- **A reescrita de PNG versionado por `csharp-ls.spec.ts`, `find-usages.spec.ts` e `zy-gutter.spec.ts`
  não foi executada nesta sessão** (specs lentos e/ou quarentenados). A tabela acima é leitura do
  código mais `git ls-files`, não medição. O que foi medido é a classe do defeito: run que falha
  gravando captura, e captura escrita onde a variável aponta.
- Windows e macOS Intel: não executados.
- App Tauri real: não se aplica (a mudança é do harness de E2E; nenhuma superfície do shell mudou).
