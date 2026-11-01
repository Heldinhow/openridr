# 35 — Debug sem modal de consentimento

Fonte: [SPEC-ALPHA3-DEBUG-TRUST](../SPEC-ALPHA3-DEBUG-TRUST.md), D1. Branch: `fix/debug-no-prompt-trust-ux`.

- [x] Remover `debugConsent`/modal `debug-consent-dialog` de `useExecutionFlow.ts` e `WorkspaceView.tsx`; nenhum "Enable and debug" no código.
- [x] `prepareAndDebug` (sem parâmetro de consentimento) ativa `debug.flag {enabled:true, confirm:true}` quando macOS arm64 e flag off, faz `debug.plan`/`debug.install` (reuso/instalação só se faltar), espera o adaptador e chama `debug.start`; erros viram mensagem no painel. Sem pergunta.
- [x] Atalho/ação `debug.start`, botão da toolbar e botão `debug-launch` do painel usam o fluxo automático. O botão do painel deixa de ficar desabilitado só porque o adaptador ainda não está pronto; continua bloqueado em Restricted mode, build em andamento e sessão ativa.
- [x] Painel Debug mostra aviso não bloqueante `dap-experimental-note` quando `appleSiliconExperimental` (Apple Silicon, experimental, suporte da comunidade, versão aprovada 3.2.0-1092). Sem modal.
- [x] Specs: `debugger.spec.ts` e `ui-fold.spec.ts` não clicam mais consentimento e afirmam `debug-consent-dialog` ausente; `gates.spec.ts`/`execution-gates.spec.ts` ajustados se citarem o fluxo antigo.
- [x] Docs: nova entrada em `decisions.md` (substitui a decisão de 2026-10-31 sobre o diálogo), `RIDR-VS-RIDER.md`, `acceptance-matrix.md`, nota histórica em `docs/evidence/auto-build-run-debug.md` + evidência nova, `implementation-notes.md`.

Aceite: `bun run check`; `cargo test -p openridr-host` (se tocar host; esperado: não tocar); E2E real de Debug com netcoredbg presente chega a `Paused` sem nenhum clique intermediário.

Revisão: `cargo test -p openridr-host` passou a rodar com app home temporário por processo (não toca `~/Library/Application Support/OpenRidr`); a nota de evidência traz mtime/SHA-256 idênticos antes/depois. Detalhes em `docs/evidence/alpha3-debug-trust.md`.
