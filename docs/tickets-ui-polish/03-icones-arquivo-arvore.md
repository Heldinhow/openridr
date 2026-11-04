# 03: Ícones de arquivo e árvore

**What to build:** `FileIcon.tsx` deixa de usar texto CSS: `.cs` e projeto C# → glifo C# neutro; `.csproj/.props/.targets` → `box`; `.sln/.slnx` → `blocks`; `.json` → `braces`; `.md/.txt` → `file-text`; demais → `file`. Remove `.cs-badge`/`.json-badge` de `src/styles.css`. Cor neutra (`--muted`, `--fg` na linha selecionada). Nunca usar o logo do Visual Studio nem o hexágono do C#.

**Blocked by:** 01.

**Status:** implementado (commit local; ver RETRO-UI-POLISH).

- [ ] `grep -n "cs-badge\|json-badge" src` vazio.
- [ ] Árvore do Gabagool: projeto, `.cs`, `.csproj`, `.sln`, `.json` e pasta distinguíveis sem ler o nome (captura em `docs/evidence/ui-polish/`).
- [ ] Aba, breadcrumb e Find Usages usam o mesmo componente.
- [ ] Nenhum ícone de arquivo colorido.
- [ ] Linha selecionada e foco legíveis (contraste do glifo ≥ 3:1).
