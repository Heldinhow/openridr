# Tickets — Paridade com o Rider (G01–G23)

Fonte: `docs/SPEC-RIDER-PARITY.md`. Um commit local por ticket, com
`scripts/commit-only.sh "msg" <paths>`; sem push, merge, PR ou release.

Ordem: P0 → P1 → P2. Os tickets ativos não têm blockers entre si; os adiados (13–16) são
registros de backlog e não bloqueiam os demais.

| Ticket | Gaps | Entrega | Status |
|---|---|---|---|
| 01 | G01 | aviso de análise acionável e consulta pendente | ready-for-agent |
| 02 | G02, G03, G04, G05 | keymap: decisões, Search Everywhere e janelas | ready-for-agent |
| 03 | G08 | descoberta automática de testes | ready-for-agent |
| 04 | G09 | `arquivo:linha` clicável na stack trace | ready-for-agent |
| 05 | G13, G14, G15 | menus da árvore e da solução + Show in Finder | ready-for-agent |
| 06 | G20 | links de localização no Build output | ready-for-agent |
| 07 | G06 | foco no editor após navegação | ready-for-agent |
| 08 | G07 | contador `n de m` no Find do editor | ready-for-agent |
| 09 | G11, G18 | decisões de chords e configuração derivada | ready-for-agent |
| 10 | G12, G17 | próxima falha e Run Unit Test por tipo | ready-for-agent |
| 11 | G19 | breakpoint em linha sem código com aviso | ready-for-agent |
| 12 | G21, G22, G23 | rótulo do keymap, tooltips e ícones | ready-for-agent |
| 13 | G10 | debug de teste (attach ao test host) | adiado |
| 14 | G16 | Add Existing Item | adiado |
| 15 | G16 | Move | adiado |
| 16 | G16 | Exclude | adiado |

## Regras desta onda

- **Aceite no app real.** Cada ticket ativo só fecha com uma passada no app desktop via
  `agent-desktop`; Playwright novo fica para contrato de host ou regressão estável. O que não
  rodar entra como "não verificado" com motivo em `docs/evidence/rider-parity.md`.
- **Play e Debug verdes.** As falhas conhecidas de `docs/RETRO-DEVFLOW-2026-11-02.md` e do
  `AGENTS.md` não contam; qualquer outra falha é investigada. Falha de Debug causada pelo
  ambiente de `fix/debug-dotnet-path` fica bloqueada por ela, sem contorno.
- **Sem dependência nova.** Só as já autorizadas (lucide-react e fontes). Nada de screenshots,
  código ou strings copiadas do Rider.
- **Idioma.** Tickets em pt-BR; UI e comentários de código em inglês; commits com assunto
  convencional em inglês.
- **Fora.** `DESIGN.md` e `PRODUCT.md` não são tocados; a branch `fix/debug-dotnet-path` não é
  importada nem mesclada.
