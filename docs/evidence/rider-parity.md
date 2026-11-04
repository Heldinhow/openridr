# Evidência — Paridade com o Rider (tickets 01–12)

Data: 2026-11-04. Branch `feat/rider-parity`. Plataforma: macOS ARM64.
Tickets 13–16 estão adiados e não foram verificados nesta rodada.

## Resumo

- **Playwright (superfície A, Chrome, host isolado):** 115 passaram, 2 falharam, 1 pulado,
  em 118 testes dos specs ligados aos tickets 01–12.
- **Falhas:** as 2 de `tests/e2e/run-configs.spec.ts` estão na lista de falhas conhecidas do
  HEAD (`AGENTS.md`). Não são causadas por esta onda.
- **App Tauri real (superfície C, agent-desktop):** não verificado. O bundle de debug foi
  gerado, mas uma instância de OpenRidr de outro worktree está aberta e bloqueia o
  `desktop.sh`. Permissões de Acessibilidade e Gravação de Tela estão concedidas.
- Nenhum ticket foi marcado como verificado no app real.

## Como foi rodado

```sh
OPENRIDR_E2E_UI_PORT=1430 OPENRIDR_E2E_HOST_PORT=4327 bunx playwright test <specs> --reporter=line
```

- Portas alternativas porque a 1420 estava ocupada por um `node` de outra sessão, que não foi
  encerrado.
- Specs: `analysis-gate`, `install-plan`, `find-usages`, `tests-auto-discovery`, `tests`,
  `editor-find-count`, `focus-after-navigation`, `breakpoint-no-code`, `run-configs`,
  `disabled-states`, `shell`, `settings`, `build`, `solution-explorer`, `actions`,
  `medium-nav-search`, `medium-nav-tabs-keymap`, `medium-editor-state`, `run`, `debugger`,
  `trust-prompt`, `git`, `search`.
- Primeira tentativa foi interrompida antes do fim; esta é a rodada completa.
- Log bruto em `.scratch/verify/e2e-run2.log` (não versionado).

## Falhas e pulo

- `run-configs.spec.ts:139` — "dialog Run does not launch its requested configuration when
  rebuild fails". Falha conhecida do HEAD.
- `run-configs.spec.ts:171` — "dialog Run refuses a failed saved build". Falha conhecida do HEAD.
  Não investigado nesta rodada: a causa exata não foi lida no log.
- **1 teste pulado, não identificado.** O reporter de linha não nomeia o pulado. Os specs que
  pulam por `csharp-ls` ausente no home de E2E (`find-usages`, `medium-nav-search`) são os
  candidatos. Consequência: o caminho "Start analysis" de Find Usages (ticket 01) pode não ter
  sido exercitado. Ver "Pendências".

## Por ticket

Legenda: **PW** = provado pelo Playwright nesta rodada (ou no spec citado); **sem evidência
visual** = não rodado no app real; **não verificado** = depende de ferramenta ausente ou de
spec pulado.

| Ticket | Gaps | Prova nesta rodada | App real |
|---|---|---|---|
| 01 análise | G01 | PW `analysis-gate`, `install-plan`; `find-usages` (caso "Start" pode ter sido o pulado) | sem evidência visual |
| 02 keymap e janelas | G02–G05 | PW `medium-nav-tabs-keymap` (⌘T, ⇧⇧, ⌘1–⌘8, rótulos na Settings), `settings` | sem evidência visual (duplo Shift, ⌘T, ⌘1, ⌘8 só no Chrome) |
| 03 descoberta automática | G08 | PW `tests-auto-discovery`, `tests`, `testsuite` | sem evidência visual |
| 04 stack trace clicável | G09 | PW `tests` (link `arquivo:linha` abre a linha) | sem evidência visual |
| 05 menus e Finder | G13–G15 | PW `solution-explorer` (menu de pasta, menu da solução, reveal fora do workspace) | sem evidência visual (Finder não verificado) |
| 06 build clicável | G20 | PW `build` | sem evidência visual |
| 07 foco após navegação | G06 | PW `focus-after-navigation` | sem evidência visual |
| 08 contador do Find | G07 | PW `editor-find-count` | sem evidência visual |
| 09 chords e run config | G11, G18 | PW `run-configs` (o "Duplicate to edit" passa; 2 falhas conhecidas no mesmo arquivo), `settings` | sem evidência visual |
| 10 próxima falha e Run Unit Test | G12, G17 | PW `tests`, `run` | sem evidência visual |
| 11 breakpoint sem código | G19 | PW `breakpoint-no-code` | sem evidência visual |
| 12 chrome e rótulos | G21–G23 | PW `shell`, `disabled-states`, `medium-nav-tabs-keymap` | sem evidência visual |

Os itens "Evidência no app real" marcados como pendentes nos tickets continuam abertos.

## Find Usages com csharp-ls (ticket 01)

`csharp-ls` já está no home de E2E (`tools/csharp-ls`). `tests/e2e/find-usages.spec.ts`, rodado
sozinho com o reporter `list`, em 2 execuções:

- "a blocked Find Usages runs once after Start analysis": **passou** nas duas. É a prova de
  "Start analysis" do ticket 01: o aviso mostra "Start analysis", a ação leva a `Ready` ou
  `Partial coverage`, e a consulta pendente roda.
- "Find Usages lists the real references of Order and opens one": **falhou nas duas**, com
  `lsp-state` em "Partial coverage" quando o teste espera "Ready" (timeout de 90 s). Não
  investigado. O AGENTS.md lista `find-usages` como intermitente no HEAD; aqui falhou 2 de 2,
  então não é intermitência confirmada nesta máquina. Não comprovei se já falhava antes das
  mudanças desta branch.

O teste pulado da rodada completa era este mesmo ambiente de `csharp-ls`; ele não ficou
pulado nesta execução separada.

## Bundle de debug e app real

- Build: `bun run tauri build --debug --bundles app` concluiu (exit 0). Bundle em
  `src-tauri/target/debug/bundle/macos/OpenRidr.app`. Não foi tocado `/Applications`.
- **Bloqueio do aceite no app real:** `pgrep -x openridr` mostra uma instância já rodando (PID
  58909, lançada por outro worktree, `openridr-ui-polish-cc`). O `desktop.sh up` recusa iniciar
  outra para não dirigir a instância de outra pessoa. Não encerrei esse processo. Para seguir,
  é preciso fechar essa instância.
- Permissões: Acessibilidade `granted`; Gravação de Tela `granted` (checado depois da concessão).
- O bundle de `/Applications` estava em alpha.4; o `package.json` está em alpha.6. O bundle novo
  é da versão atual do checkout.

## Pendências

1. **Aceite no app real (agent-desktop):** bloqueado pela instância aberta (PID 58909). Depois
   de fechá-la: `desktop.sh up src-tauri/target/debug/bundle/macos/OpenRidr.app`, depois os
   roteiros de cada ticket. Nada disso foi executado ainda.
2. **verify-openridr, superfície B (navegador):** não executada. A ferramenta de navegador da
   skill não está disponível nesta sessão.
3. **Falhas conhecidas:** `run-configs.spec.ts` (2) e `find-usages` "Partial coverage" (1)
   seguem sem investigação aqui.
