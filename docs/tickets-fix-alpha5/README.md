# Tickets — correções do E2E alpha.5

Fonte: [`../SPEC-FIX-ALPHA5.md`](../SPEC-FIX-ALPHA5.md) e [`../evidence/e2e-alpha5/REPORT.md`](../evidence/e2e-alpha5/REPORT.md).

Ordem de dependência: 01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09 → 10 → 11 → 12 → 13 → 14 → 15.
Bloqueios: 04←03, 09←05, 13←12, 14←06+12, 15←06.

| # | Ticket | Bugs | Bloqueado por |
|---|---|---|---|
| 01 | [Quit pergunta antes de fechar](01-quit-unsaved.md) | BUG-001 | — |
| 02 | [Debug após mudança externa de fonte](02-debug-source-gate.md) | BUG-002 | — |
| 03 | [CRLF e UTF-16 preservados](03-encoding-crlf-utf16.md) | BUG-005, BUG-006 | — |
| 04 | [Buffer limpo recarrega sozinho](04-clean-buffer-reload.md) | BUG-003 | 03 |
| 05 | [Árvore reflete mudanças externas](05-tree-external-freshness.md) | BUG-004 | — |
| 06 | [Debugger instalado com transparência](06-debugger-install-ux.md) | BUG-008, BUG-015 | — |
| 07 | [Mensagens honestas de abertura e avaliação](07-honest-messages.md) | BUG-007, BUG-016, BUG-017 | — |
| 08 | [Nomes acessíveis inequívocos](08-accessible-names.md) | BUG-018 | — |
| 09 | [Criar, renomear e apagar arquivos](09-tree-file-operations.md) | BUG-019 | 05 |
| 10 | [Instância única](10-single-instance.md) | BUG-014 | — |
| 11 | [verify-openridr honesta](11-verify-skill-hardening.md) | BUG-009…013, BUG-020 | — |
| 12 | [SDK do projeto x runtime das ferramentas](12-sdk-runtime-split.md) | camada SDK | — |
| 13 | [Mensagem por projeto e links vivos](13-project-sdk-messaging.md) | camada SDK | 12 |
| 14 | [Download do csharp-ls com progresso/retry](14-csharp-ls-download-ux.md) | camada SDK | 06, 12 |
| 15 | [Proposta de embutir o netcoredbg](15-netcoredbg-embed-proposal.md) | follow-up | 06 |

Estado real: `git log` desta branch e as notas em `docs/implementation-notes.md`. Specs/tickets antigos mantêm `ready-for-agent` no cabeçalho por convenção do repositório.
