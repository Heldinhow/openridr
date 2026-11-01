# Tickets — MVP Rider-like (F1–F9)

## Escopo e autorização

Fonte normativa: [SPEC-MVP-RIDER](../SPEC-MVP-RIDER.md). Decisões: [decisions.md](../decisions.md) e o grill (`~/.opencode/plan/openridr-ui/GRILL-MVP.md`, D1–D16). Handoff: `~/.opencode/plan/openridr-ui/HANDOFF-mvp-rider.md`.

Tracker local em `docs/tickets-mvp/` por decisão do Helder. Cada ticket é uma fatia vertical: host (quando houver), UI, testes e evidência na mesma entrega. Um ticket por fatia, na ordem de dependência; a ordem das fatias F1–F9 foi aprovada no grill e não está em votação.

Não autoriza push, PR, merge em remote, novo repositório nem instalação de ferramenta/SDK. Commits locais por ticket, em branch local de integração.

## Índice e grafo

| Ticket | Fatia | Bloqueado por | Entrega demonstrável |
|---|---|---|---|
| [01 — Casca com tool windows, faixas e slots](01-f1-casca.md) | F1 | Nenhum | Painéis existentes viram tool windows alternáveis, com barra única e status bar |
| [02 — Registro de ações e keymap](02-f2-acoes-keymap.md) | F2 | 01 | Toda ação vem do registro; Find Action executa por nome; atalhos por perfil |
| [03 — Busca global](03-f3-busca.md) | F3 | 02 | Search Everywhere com 6 abas, símbolos reais e busca textual |
| [04 — Run configurations](04-f4-run-configurations.md) | F4 | 01 | Configuração salva por solução, dialogada e ligada a Run/Debug |
| [05 — Tool window Tests](05-f5-tests.md) | F5 | 01 | Árvore, filtro, run/debug/rerun failed e quick list |
| [06 — Git](06-f6-git.md) | F6 | 01 | Branch, commit seletivo, diff, log, pull/push confirmados |
| [07 — Settings](07-f8-settings.md) | F8 | 02 | Settings com busca, keymap, editor, SDK e VCS |
| [08 — NuGet](08-f9-nuget.md) | F9 | 01 | Instalados/desatualizados e update confirmado, sem restore implícito |
| [09 — Código: gutter, ⌥↩, menu, rename, usages](09-f7-codigo.md) | F7 | 02, 05, 06 | Gutter Run/Test, Alt+Enter com prévia, menu de contexto, rename inline, Find Usages |

## Pós-MVP — correções da auditoria visual (UI-AUDIT)

Fonte: [UI-AUDIT.md](../UI-AUDIT.md), auditoria visual Chrome 155 em 29/10/2026. Estes tickets corrigem apresentação/ergonomia sem alterar contratos funcionais do MVP. Evidências e limitações estão no audit. Ordem de entrega: 19 → 20 → 21 → 22 → 23 → 24 → 25 → 26; cada ticket é uma unidade independente e exige commit local próprio.

| Ticket | Escopo | Desvios | Bloqueado por |
|---|---|---|---|
| [19 — Saída e hierarquia de Build/Run/Tests/Debug](19-audit-run-tests-debug.md) | Resultado principal visível nos painéis de execução | A1, A2, L1, L2 | Nenhum |
| [20 — Git: diff, caminhos e log legíveis](20-audit-git.md) | Revisão de diff e leitura de arquivos/histórico | A3, M8, L5 | Nenhum |
| [21 — Estados disabled e planos de ferramentas](21-audit-disabled-tools.md) | Aparência indisponível e conteúdo material antes de confirmar | A4, A5, M1, M6 | Nenhum |
| [22 — Editor: popups, rename e ações responsivas](22-audit-editor.md) | Posicionamento/visibilidade de ações de código e controles do editor | A6, M7, M13, L6, L9 | Nenhum |
| [23 — Layout, detalhes e feedback transversal](23-audit-layout.md) | Barra/status em telas estreitas, Problems, project details, terminal e diálogos | A7, M2, M5, M10, M12, L3, L7 | Nenhum |
| [24 — NuGet: estados sem pacotes e tabela](24-audit-nuget.md) | Parser de ausência de frameworks e legibilidade da lista/fontes | A8, M9 | Nenhum |
| [25 — Settings e Keymap](25-audit-settings-keymap.md) | Tabela de atalhos, caminhos/configuração e tema indisponível | M4, M11, L8 | Nenhum |
| [26 — Busca e workspace tools: dicas de descoberta](26-audit-discovery.md) | Contagem e sugestão contextual de busca | L4 | Nenhum |

Ordem de implementação: **01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09** (F1, F2, F3, F4, F5, F6, F8, F9, F7).

## Alfa 3 — Debug, confiança e dados de teste (2026-11-01)

Fonte: [SPEC-ALPHA3-DEBUG-TRUST](../SPEC-ALPHA3-DEBUG-TRUST.md). Três fatias independentes, entregues nesta ordem; cada uma com commit local próprio.

| Ticket | Escopo | Bloqueado por |
|---|---|---|
| [35 — Debug sem modal de consentimento](35-debug-sem-modal.md) | Clique em Debug liga a flag, reusa/instala o adaptador e inicia; sem diálogo | Nenhum |
| [36 — Confiar no próprio aviso de bloqueio](36-confiar-no-aviso.md) | Avisos de Build/Run/Debug em Restricted oferecem "Trust this project" | Nenhum |
| [37 — Dados de teste isolados e higiene](37-dados-de-teste-isolados.md) | Host E2E em home temporário, `host:http` fora do app home, `.gitignore` | Nenhum |

## Gates válidos em todos os tickets

- **Testids são contrato:** nenhum `data-testid` existente é renomeado ou removido; a suíte E2E inteira roda ao fim de cada fatia.
- **Gates do host:** todo método novo que executa algo passa por `trust_required`; respostas tardias não substituem contexto (`stale_context`); nada faz restore implícito; nenhum estado desconhecido vira sucesso.
- **Estados honestos:** `stale`, `sealed`, `partial`, `unknown`, `truncated` e "não verificado" aparecem como tais.
- **UI em inglês, docs em PT-BR.** Acessibilidade: nome acessível, foco visível, motivo textual em ação bloqueada.
- **Sem dependência nova.** Ícones SVG próprios; estado na view existente.
- **Verificação por ticket:** `bunx tsc --noEmit`, `cargo test -p openridr-host` (quando tocar host) e suíte Playwright completa ao fim da fatia; evidência só quando houver prova real.
- **Commit local por ticket**, mensagem `feat(FN): <entrega>`; sem push.
