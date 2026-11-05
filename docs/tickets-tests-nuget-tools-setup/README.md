# Tickets — Tests, NuGet e setup

Fonte: `docs/SPEC-TESTS-NUGET-TOOLS-SETUP.md`. Cada fatia exige testes e evidência; não fechar por inspeção estática.

| Ticket | Entrega | Bloqueado por | Status |
|---|---|---|---|
| 01 | Setup obrigatório, verificação, retry, trust e análise automática | — | concluído (2026-11-05); runtime ausente no app e Windows/Intel não verificados |
| 02 | Tests: estados, detalhe, expansão e exportação | — | concluído (2026-11-05); sessão de solução, debug de teste, Continuous Testing e cobertura ficam desabilitados por falta de capacidade |
| 03 | NuGet: solução, assets e updates em lote | — | concluído (2026-11-05); update em lote e escopo Solução não executados no app real |
| 04 | NuGet: fontes, prerelease e Discover | 03 | concluído (2026-11-05); Discover real verificado no app; instalação descoberta não confirmada contra a fixture |
| 05 | Verificação desktop e retrospectiva | 01, 02, 03, 04 | concluído em parte: capturas e E2E em `evidence/tests-nuget-tools-setup/desktop.md`; pendências explícitas acima |

Evidência: [desktop.md](../evidence/tests-nuget-tools-setup/desktop.md), [offline-install.md](../evidence/tests-nuget-tools-setup/offline-install.md).
Decisões: `docs/decisions.md` (2026-11-05, “Ferramentas obrigatórias no setup” e entradas seguintes).
