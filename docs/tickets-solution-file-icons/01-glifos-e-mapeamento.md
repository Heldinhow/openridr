# 01 — Glifos vendorizados e mapeamento por arquivo

Status: implementado
Spec: [D1, D2, D6, D7](../SPEC-SOLUTION-FILE-ICONS.md)

## Mudança

- `src/icons/fileGlyphs.tsx` (novo): SVGs vendorizados do `@pierre/trees` 1.0.0-beta.6, Apache-2.0
  (`doc`, `json`, `markdown`, `docker`, `yml`, `text`, `git`) + o hexágono Simple Icons (`C#`, CC0) tal
  como distribuído no sprite do T3 Code. Cada um é um componente React inline de 16 px; nada de sprite
  global, nenhuma dependência nova.
- `src/icons/CSharpIcon.tsx`: passa a ser o hexágono preenchido (antes era o glifo de traço do MDI).
- `src/icons/SymbolMark.tsx` (novo, substitui `SymbolIcon.tsx`): selo de letra 9 px sobreposto ao hexágono.
- `src/FileIcon.tsx`: escolhe glifo por `fileGlyph(fileKind(path))`; o `.cs` sempre usa o hexágono e o selo
  de símbolo é opcional (`title` com o tipo). Sai o ramo que trocava o ícone do projeto por `ProjectIcon`
  — o `.csproj` de Show all files passa a ser documento (o projeto na árvore continua com `ProjectIcon`).
- `src/solutionVisuals.ts`: kinds novos `docker`, `dotsettings`, `git`; resolução por nome antes da
  extensão (`Dockerfile`, `docker-compose*.yml`, `*.DotSettings[.user]`, `.gitignore`/`.gitattributes`/
  `.gitmodules`, `.editorconfig`, `global.json`, `appsettings*.json`) e `fileGlyph(kind)` puro.
- `src/styles.css`: tokens novos `--icon-doc`, `--icon-docker`, `--icon-git`; `--icon-csharp`,
  `--icon-json`, `--icon-markdown` trocados pelos hex do T3 Code; `.file-glyph`, `.glyph-page` e
  `.symbol-mark`; regras `.tint-*` para os kinds novos e remoção do `.symbol-badge` (15 px) antigo.

## Testes

- `tests/unit/solutionVisuals.test.ts`: cobre nome/extensão → kind e kind → glifo, incluindo os casos da
  tabela da spec e desconhecido/genérico.

## Aceite

Nenhuma dependência nova; `bun run check` e `bun run test:unit` verdes; cada glifo tem origem e licença em
`THIRD-PARTY-NOTICES.md`.
