# Tickets: polimento visual (ícones e tipografia)

Spec: [../SPEC-UI-POLISH.md](../SPEC-UI-POLISH.md). Diagnóstico: [../evidence/ui-polish.md](../evidence/ui-polish.md). Implementados em 2026-11-04 após as respostas do Helder (D4, D6, D7, D9).

| # | Ticket | Bloqueado por |
|---|---|---|
| 01 | [Lucide + glifo C# no lugar dos paths à mão](01-icones-lucide.md) | aprovação da dependência (D7) |
| 02 | [Ícones de depuração e execução](02-icones-execucao-debug.md) | 01 |
| 03 | [Ícones de arquivo e árvore (C#, .csproj, .sln, .json)](03-icones-arquivo-arvore.md) | 01 |
| 04 | [Escala tipográfica em 4 tokens](04-escala-tipografica.md) | — |
| 05 | [Embutir Inter e JetBrains Mono](05-fontes-embutidas.md) | aprovação da dependência (D7) |
| 06 | [Avisos de licença e verificação final](06-licencas-e-verificacao.md) | 01, 05 |

Ordem sugerida: 04 em paralelo com 01 → 02, 03 → 05 → 06. Um commit por ticket (`feat(ui): …`), via `scripts/commit-only.sh`.
