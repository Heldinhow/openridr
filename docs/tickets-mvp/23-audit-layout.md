# 23 — Auditoria visual: layout, Problems e feedback transversal

**What to build:** Corrigir truncamento/rolagem e hierarquia de informações em barra/status, Problems, detalhes do projeto, terminal e diálogos.

**Source:** `docs/UI-AUDIT.md` A7/M2/M3/M5/M10/M12/L3/L7; SPEC P0 §§98,100 e F1 #6–8/F4; decisões dos tickets 03, 06, 15 e 18.

**Blocked by:** nenhum.

**Status:** in-progress

- [ ] Barra e status permanecem legíveis em 1600×1000, 1440×900 e 1100×700: nome de projeto/configuração e status de avaliação não truncam enquanto houver espaço; overflow local prioriza ações.
- [ ] Mensagem de branch longa usa rótulo compacto e mantém motivo completo via texto acessível/tooltip.
- [ ] Problems mostra colunas de severidade/código/mensagem/origem/categoria sem cortes essenciais, destaca seleção e deixa linha selecionada/contagem alcançáveis.
- [ ] Run configurations dialog uses Run/Cancel/Apply/OK semantics; default read-only fields look read-only and duplicating an unnamed config cannot create a blank entry.
- [ ] Project details não quebra caminhos no meio de nomes de arquivo; comando longo pode ser rolado/copiad​o; Close fica associado ao cabeçalho.
- [ ] Terminal mantém cabeçalho/controles visíveis com output extenso e não apresenta eco/redraw duplicado quando possível sem fingir que é emulador; discrepância SDK do PTY continua explicitamente reportada, não mascarada.
- [ ] Diálogos de fechamento dão hierarquia neutra à ação destrutiva e destaque à ação segura/reversível; não depender só de cor.
- [ ] Caminhos internos/temporários são apresentados com rótulos úteis ou detalhes acessíveis; evitar duplicar workspace no breadcrumb.
- [ ] E2E cobre problemas selecionados, layouts nas dimensões citadas e fechamento; não mudar contratos do host nem testids.
- [ ] Executar build e E2E aplicável; ambiente/hardware e flows não exercitados são anotados como não verificados.

## Comments

- Capturas: `88-layout-1100.png`, `18-editor-program.png`, `96-problems-error.png`, `97-problem-selected.png`, `19-project-details.png`, `84-terminal.png`, `86-close-dirty.png`.
- Terminal não é um emulador completo: tratar redraw visual apenas dentro do limite do buffer atual e não adicionar dependência sem decisão aprovada.
- Implementação parcial nesta revisão: barra/status com overflow local, rótulo de Git compacto, breadcrumb sem duplicar workspace, detalhes de projeto/comando roláveis, header/controles do terminal preservados e ação destrutiva neutra. Os itens de semântica Run/Cancel/Apply/OK, eco/redraw do PTY, validação visual das colunas Problems e diálogos ainda precisam de cobertura antes de concluir o ticket.
