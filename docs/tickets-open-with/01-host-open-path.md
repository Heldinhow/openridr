# 01 — Caminho externo e `.slnx` no host

**Fonte:** `docs/SPEC-OPEN-WITH.md` §Problem/Solution/Implementation Decisions.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

`workspace.open` aceita o caminho que veio do Finder/terminal como aceita o da tela inicial:
relativo (com `baseDir` opcional), com extensão permitida sem diferenciar maiúsculas, ou
diretório com exatamente uma solution/projeto. `.slnx` abre e mostra os projetos na árvore.
Nada é executado ao abrir.

## Pontos de código

- `crates/host/src/lib.rs`:
  - `workspace.open`: resolver `path` + `baseDir` antes de canonicalizar. `baseDir` opcional
    precisa ser absoluto; relativo sem `baseDir` usa `std::env::current_dir()`. A canonicalização
    continua sendo a validação de existência.
  - `workspace_file`: allowlist `.sln`, `.slnx`, `.csproj` com comparação case-insensitive.
  - `resolve_directory`: juntar `.sln` e `.slnx` como solutions; manter a regra de um único
    `.csproj`.
  - `load_workspace`: `.slnx` é `WorkspaceKind::Solution`.
  - `pick_workspace` já lista `slnx`; não mudar o diálogo.
- `crates/host/src/outline.rs`: parser de `.slnx` para `<Project ... Path="..."/>` (aspas simples
  ou duplas, tag em uma ou várias linhas). Reusar `outline_project`/`normalize_rel`; o caminho
  continua confinado à raiz.
- Não adicionar dependência de XML.

## Teste primeiro

1. Em `crates/host/src/lib.rs` (`mod tests`), unidades:
   - `workspace.open` com caminho relativo e `baseDir` abre a fixture.
   - `.SLN` maiúsculo abre (case-insensitive).
   - `.txt` existente devolve `path_not_allowed` e não troca o workspace aberto.
   - caminho inexistente devolve `not_found`.
   - diretório com duas solutions devolve `ambiguous_workspace` e candidatos.
2. Em `crates/host/src/outline.rs`, unidades: `.slnx` compacto e multi-linha devolvem os
   projetos na ordem do arquivo; `<Folder>` não vira projeto.
3. Novo `tests/e2e/open-external.spec.ts` contra a ponte HTTP:
   - `workspace.open {path: "fixtures/OrderDesk/OrderDesk.sln", baseDir: <raiz do repo>}`
     devolve `kind: "solution"` e caminho canonicalizado;
   - `.csproj` da fixture devolve `kind: "project"`;
   - `.slnx` temporário (criado no teste) devolve os projetos;
   - diretório temporário com dois `.sln` devolve `ambiguous_workspace`;
   - `package.json` existente devolve `path_not_allowed` e o workspace anterior continua.
   Fechar o workspace ao final (o `afterEach` do spec fecha).

Rodar os testes e ver falhar pelo motivo certo antes de implementar:
`cargo test -p openridr-host workspace_open` (ou o filtro escolhido) e
`bunx playwright test tests/e2e/open-external.spec.ts`.

## Aceite

- [x] `workspace.open` relativo/absoluto, extensão case-insensitive e `baseDir` validados.
- [x] `.slnx` abre como solution e o outline lista os projetos.
- [x] `.csproj` continua `kind: project`.
- [x] Diretório ambíguo continua recusando com candidatos.
- [x] Nenhum processo/execute ao abrir; nenhuma dependência nova.
- [x] Unidades (201/201) e E2E alvo verdes; `bun run check` verde; `cargo test -p openridr-host`
      com a única falha pré-existente `file_access_is_confined_to_the_workspace`.
- [x] Entrada de 2026-11-02 em `docs/decisions.md` já cobre as decisões (ajustar só se o
      código provar outra coisa).
- [x] Commit local `feat(open): accept external and .slnx workspace paths` só com os caminhos
      deste ticket.
