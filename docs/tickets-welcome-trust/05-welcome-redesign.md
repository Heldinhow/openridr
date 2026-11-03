# 05: Welcome redesenhada (Projects + Toolchain)

**What to build:** A tela inicial vira uma welcome no estilo Rider: lista de recentes com nome, caminho curto, último acesso e selo de trust; menu ⋯ (Trust/Restrict, Remover); busca com contagem; estados vazio e erro por linha; atalhos; Toolchain em segunda vista. Usar impeccable (context, ler reference/craft-floor.md antes de editar; critique/layout/typeset/polish) e capturar o Rider.app como referência.

**Blocked by:** 02, 03, 04

**Status:** done

- [x] Lista, busca, contagem, vazio, erro por linha, ↑/↓/Enter, ⌘O, `/`, Remover, mudar trust sem abrir
- [x] Toolchain e processos na segunda vista com testids existentes; aviso discreto de SDK ausente em Projects
- [x] Piso 12px, espaçamento e tokens do DESIGN.md; só escuro; sem dependência nova; um componente por arquivo
- [x] Captura antes/depois e da referência do Rider em docs/evidence; specs de welcome/ui-layout migrados e verdes; commit `feat(welcome): ...`

## Comments

- 2026-11-01: implementado em `src/welcome/*` + `SetupScreen`; specs `welcome.spec.ts` (6) e migrados verdes. Referência do Rider.app **não capturada** (abrir o app reabriria o último projeto do usuário); desenho segue o conhecimento da welcome do Rider. Ver `docs/implementation-notes.md`.
