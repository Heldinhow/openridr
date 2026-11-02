# 03 — Erros da descoberta de testes em Problems (d7)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §3. **Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Erro de compilação do projeto de testes durante `tests.discover` aparece em Problems,
clicável, como o erro de build; o log cru continua.

## Pontos de código

- `crates/host/src/build.rs`: extrair o registro de problema (dedupe + `id` + `build_id` +
  `revision` + limite 22) para uma função pública reutilizável; `Build::append_line` passa a
  usá-la sem mudar o formato.
- `crates/host/src/test_discovery.rs`: `Start` ganha `root: PathBuf`; `Data`/`DiscoveryView`
  ganham `problems: Vec<BuildProblem>` e `problems_truncated: bool`; `append_line` chama o
  parser para stdout e stderr com a raiz do workspace. `build_id` = discovery id.
- `crates/host/src/lib.rs`: preencher `root` no início da descoberta (raiz do workspace, não o
  cwd do projeto) e incluir os campos no snapshot (o snapshot já publica `tests.view()`).
- `src/host/client.ts`: `TestsDiscovery.problems`/`problemsTruncated`.
- `src/ProblemsPanel.tsx`: nova seção `tests-problems` com contagem e linhas
  `tests-problem-row-N` (mesma navegação/`data-position` das linhas de build);
  `src/WorkspaceView.tsx` passa `snapshot.tests`.

## Teste primeiro

`tests/e2e/tests.spec.ts`: copiar OrderDesk, inserir erro de sintaxe em
`tests/OrderDesk.Tests/OrderTests.cs` (`this is not c#`), confiar/restaurar, abrir Tests,
`tests-discover`; esperar `tests-state` falha e, em Problems, `tests-problems` com um
`tests-problem-row-0` contendo `CS`; clicar e conferir `editor-shell[data-cursor]` no arquivo
de testes. Falhar antes da correção (Problems hoje só tem build).

## Aceite

- [ ] Descoberta falha por compilação publica `problems` no host.
- [ ] Problems lista e navega; log cru intacto.
- [ ] Nenhuma mudança no formato de `build.problems`.
- [ ] `bun run check` verde. Commit `fix(tests): publish discovery compile problems`.
