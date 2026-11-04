# 01 — Análise a partir do aviso (G01)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Análise e Find Usages". **Bloqueado por:**
nenhum. **Status:** ready-for-agent.

## Objetivo

Find Usages, Rename e code actions não podem terminar num aviso morto quando a análise não
está pronta. O aviso oferece instalar (com plano revisável) ou iniciar a análise, e a consulta
pendente roda sozinha quando a análise fica ativa.

## Pontos de código

- `src/WorkspaceView.tsx`: estado de `message`, `findUsagesDisabledReason`,
  `renameDisabledReason`, `codeActionsDisabledReason`, `startAnalysis`, abertura do painel de
  ferramentas e o efeito de auto-start da análise.
- `src/ToolchainPanels.tsx`: `ToolInstaller` (pedido do plano), `LspPanel`, `lspStartBlocker`.
- `src/FindUsagesPanel.tsx`: estado de resposta indisponível, se o aviso também precisar
  aparecer dentro do painel.
- `src/styles.css`: ação opcional na barra de aviso.

## Comportamento

1. Com csharp-ls ausente e workspace confiável, acionar Find Usages mostra um aviso com
   "Install analysis". A ação abre o painel de ferramentas e solicita `tools.plan`; nada é
   baixado nem instalado antes do "Confirm and install" existente.
2. Com csharp-ls pronto e análise parada, o aviso mostra "Start analysis"; a ação chama o
   mesmo `lsp.start` do painel.
3. Quando a análise fica ativa, a última ação bloqueada (Find Usages, Rename ou code action)
   roda uma vez sozinha; se falhar, a mensagem normal aparece.
4. Em modo restrito o bloqueio continua sendo trust; não se oferece instalação como saída.
5. O mesmo aviso é usado pelas três ações; não há três cópias de texto.

## Teste e verificação

- Playwright: `tests/e2e/find-usages.spec.ts` ganha o caso "sem análise → aviso acionável" e
  confere que "Install analysis" abre o plano sem `tools.install`; o caso de instalação real
  só roda com csharp-ls disponível e autorização, senão `skip` com motivo.
- App real (agent-desktop): workspace confiável sem análise; ⇧F12 num símbolo; conferir o
  aviso, abrir o plano e cancelar; com a análise já instalada, parar e usar "Start analysis";
  conferir que Find Usages volta sozinho. Se a instalação real não for autorizada, registrar
  "não verificado" para o trecho pós-instalação.

## Aceite

- [x] Find Usages num símbolo, sem análise, mostra ação "Install analysis" ou "Start analysis"
      no próprio aviso. — Playwright `analysis-gate.spec.ts` (Install) e `find-usages.spec.ts`
      (Start).
- [x] "Install analysis" abre o painel com o plano solicitado e nenhum download antes da
      confirmação. — `analysis-gate.spec.ts`: plano `planned`, sem processo/progresso.
- [x] Com a análise ativa, a consulta pendente roda uma vez e lista as ocorrências. — caso
      "a blocked Find Usages runs once after Start analysis" em `find-usages.spec.ts`.
- [x] Rename e code actions mostram o mesmo aviso e as mesmas ações. — mesmo spec cobre ⇧F12,
      ⇧F6 e ⌥Enter.
- [x] Restricted mode continua bloqueando por trust, sem oferecer instalação. — mesmo spec,
      primeiro caso.
- [ ] Evidência no app real (ou "não verificado" com motivo) em `docs/evidence/rider-parity.md`.
      **Não verificado nesta rodada:** a passada no app Tauri real (agent-desktop) fica para a
      varredura final com `verify-openridr`; o download real também não foi autorizado/executado.

