# E2E — conjunto de smoke e quarentena (issue #67)

Decisão do Helder (2026-11-06, issue #67): **o smoke noturno é o único E2E automatizado** e **nunca
roda em PR**. O workflow noturno que consome a lista é a issue #56; este ticket entrega só a lista e
a quarentena.

Verificação de UI por sessão de agente é Playwright em modo web, só os specs da área (decisão do
Helder de 2026-11-07, issue #212; ver [SPEC-E2E-PORTS](../SPEC-E2E-PORTS.md)); o app Tauri real com
`agent-desktop` fica para o que é nativo do shell.

## Conjunto de smoke

Fonte da lista: [`scripts/e2e-smoke.txt`](../../scripts/e2e-smoke.txt) (um spec por linha; a #56 lê
esse arquivo). Cobre o caminho que mais dói quebrar: abrir solução/trust/editar (`workspace`),
Build, Run e Problems.

| Spec | Testes | Por que está no smoke |
|---|---|---|
| `tests/e2e/workspace.spec.ts` | 16 | abrir solução, Trust/Restricted, editar/salvar, conflito de disco, erros de IO |
| `tests/e2e/build.spec.ts` | 4 | build real, log cru, erro de compilação e navegação, restore automático |
| `tests/e2e/run.spec.ts` | 5 | run real, output do build salvo, stop observado, exceção, um clique |
| `tests/e2e/qa2-diagnostics.spec.ts` | 2 | Problems no editor/gutter/status, análise real |
| `tests/e2e/solution-git-status.spec.ts` | 7 | status de Git na árvore, `.gitignore`, symlink, consulta repetida com limite |
| `tests/e2e/trust-sdk.spec.ts` | 3 | trust/revogação, SDK do projeto resolvido e `sdk_missing` distinto, processo sobrevivendo à revogação (issue #309) |

Medição (macOS ARM64, 2026-11-06, headless, `workers=1`, um host por execução): **25 testes, 25
passaram, ~2,8 min**, em duas rodadas seguidas. Remedido em 2026-11-08 com o conjunto atual (37
testes, + `solution-git-status` e `trust-sdk`): **37 passaram, 4,8 min**, portas 1920/4917. Antes disso, o teste
`workspace.spec.ts` › `opens the fixture asking for a Trust decision…` falhava porque esperava o
texto do fluxo de download (`osx-arm64 asset exists`); com o netcoredbg embutido o texto é
"Bundled with OpenRidr" e a expectativa foi corrigida (não é quarentena, é spec desatualizada).

Comando (idêntico ao que a #56 deve rodar):

```sh
bunx playwright test $(grep -v '^#' scripts/e2e-smoke.txt)
```

Pré-requisitos: rede (o `global-setup` baixa csharp-ls do nuget.org com `curl` e roda `dotnet tool
install`), Chrome (`PLAYWRIGHT_CHANNEL=chromium` onde não houver) e portas 1420/4317 livres. As
capturas caem em `<tmpdir>/openridr-e2e-screenshots-<porta da UI>`, nunca no repositório;
`OPENRIDR_SCREENSHOT_DIR` só é preciso para escrever em `docs/evidence/screenshots` de propósito.

## Quarentena

| Spec / teste | Sintoma | Motivo | Status |
|---|---|---|---|
| `tests/e2e/debugger.spec.ts` (13 testes) | **medido**: 12 falham, 1 passa, 20,8 min | escrito para o fluxo `debug.install` anterior ao bundle; o netcoredbg agora vem no app e `debug.install` é fallback de `tauri dev` | `test.beforeEach` com `test.skip(true, …)` no arquivo, com o motivo; reabrir só quando o spec for reescrito para o adaptador embutido |
| `tests/e2e/csharp-ls.spec.ts:150` (Problems após SIGSTOP/SIGCONT) | relatado na issue #43; **não re-verificado** nesta rodada | fora do conjunto de smoke; nenhuma prova desta sessão | quarentenado com motivo; reabrir com execução real antes/depois |
| `tests/e2e/testsuite.spec.ts` (`tests-filter-count` ausente) | relatado; **não re-verificado** | idem | quarentenado com motivo |
| `tests/e2e/tests.spec.ts:317` | relatado; **não re-verificado** | idem | quarentenado com motivo |
| `tests/e2e/ui-layout.spec.ts` (1440 px, timeout) | relatado; **não re-verificado** | idem | quarentenado com motivo |

Nada foi apagado. "Não re-verificado" aqui significa exatamente isso: sem execução nesta rodada,
não se declara conserto nem causa.

## O que desquarentena

- Decisão de produto que mude o fluxo testado (para `debugger.spec.ts`: reescrever o spec para o
  adaptador embutido, com o fluxo de `debug.install` só onde ele ainda existe).
- Execução real do spec com falha-antes e passa-depois registrados aqui; sem isso, continua fora.
- Novo spec que entre na lista do smoke precisa passar nas duas primeiras rodadas seguidas.
