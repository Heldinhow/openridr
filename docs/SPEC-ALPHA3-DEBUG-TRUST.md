# SPEC — alfa 3: Debug sem pergunta, confiança no bloqueio e dados de teste isolados

Data: 2026-11-01. Branch: `fix/debug-no-prompt-trust-ux`. Fonte das decisões: Helder (grill respondido).

## Problema

1. Debug no macOS Apple Silicon abre o modal `debug-consent-dialog` ("Enable the experimental debugger?") antes de fazer qualquer coisa. O Rider não pergunta: executa. O modal é o último atrito do fluxo Debug.
2. Em Restricted mode, Build/Run/Debug mostram apenas o motivo textual. Quem abriu o projeto em Restricted não descobre na hora como confiar; o controle fica na barra de status/popover e no menu da welcome.
3. Execuções de teste/manual escrevem `trusted.json` e `recent.json` no app home real (`~/Library/Application Support/OpenRidr`), acumulando pastas temporárias.
4. Arquivos de trabalho locais (`.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/`, `docs/evidence/logs/*`) precisam ficar fora do Git.

## Decisões aceitas

- **D1 — Debug direto.** Remover o modal. Clicar Debug (toolbar, atalho ou painel) ativa a flag experimental da sessão automaticamente, planeja/instala o netcoredbg 3.2.0-1092 somente se faltar, e chama `debug.start`; o host builda implicitamente quando não há build utilizável. Nenhuma pergunta. O status experimental continua visível como aviso não bloqueante no painel Debug.
- **D2 — Confiança óbvia.** Quando Build, Run ou Debug estiver bloqueado por Restricted mode, o próprio motivo mostra um botão **Trust this project** que chama `trust.grant` com `remember: true` (a escolha é persistida). Sem checkbox no aviso. O indicador de confiança continua só na barra de status (`status-mode`) e no menu "⋯" dos recentes; nada na barra superior.
- **D3 — Dados de teste isolados.** O host E2E sempre usa `OPENRIDR_HOME` temporário (`e2eHome`) e `OPENRIDR_TRUST_STORE=off`; um E2E passa a afirmar que `app.appHome` é esse diretório. A ponte manual `bun run host:http` passa a usar `.scratch/host-home` por padrão, para smoke manual não tocar o app home real.
- **D4 — Higiene.** Ignorar `.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/` e `docs/evidence/logs/*`.

## Fora de escopo

- Mudar a política de restore do host: restore continua explícito; um workspace Trusted já restaura sozinho uma vez por contexto. O Debug usa o restore automático existente e builda implicitamente.
- Remover o checkbox de `TrustControls` do popover de status (D2 só dispensa o checkbox no aviso de bloqueio).
- Limpar/reescrever os arquivos reais já acumulados em `~/Library/Application Support/OpenRidr` (dado do usuário; fica registrado como pendência).

## Aceite

- [x] Nenhuma referência ao modal `debug-consent-dialog`/botão "Enable and debug" no código; atalho, toolbar e painel Debug usam o fluxo automático.
- [x] Painel Debug mostra `dap-experimental-note` quando a flag Apple Silicon está ativa (texto não bloqueante, sem promessa de suporte oficial).
- [x] Avisos `build-disabled-reason`, `run-disabled-reason` e `dap-disabled-reason` exibem `build-trust`, `run-trust` e `dap-trust` respectivamente quando o motivo é Restricted mode; clicar concede confiança persistida e o gate cai.
- [x] `environment.spec.ts` prova `app.appHome === e2eHome`; `bun run host:http` não usa o app home real. `cargo test` também é isolado (home temporário por processo de teste).
- [x] `.gitignore` cobre os caminhos de D4; `git status` limpo deles.
- [ ] `bun run check`, `cargo test -p openridr-host`, specs tocados e docs atualizados (`decisions.md`, `RIDR-VS-RIDER.md`, `acceptance-matrix.md`, evidência, `implementation-notes.md`, retro).
