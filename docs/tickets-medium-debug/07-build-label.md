# 07 — Rótulo do Build diz "startup", não "Rebuild" (d6, contexto b21)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §7. **Bloqueado por:** 06 (opcional). **Status:** ready-for-agent.

## Objetivo

O botão do Build panel não pode prometer "Rebuild" quando o comando é um `dotnet build`
incremental do startup project. O rótulo diz o escopo e o tooltip aponta o Build Solution.

## Pontos de código

- `src/BuildPanel.tsx`: label do `build-action` = `Build startup (Cmd+F9|Ctrl+F9)`; quando o
  projeto é conhecido, `Build startup 'Nome' (… )`. Nunca "Rebuild". Tooltip: "Runs an
  incremental dotnet build of the startup project; use Build Solution (⇧⌘F9) for the whole
  solution." `build-solution` continua igual.
- `src/WorkspaceView.tsx`: tooltip do `toolbar-build` pode citar Build Solution; nenhuma
  mudança de ação/atalho (⌘F9 = `build.start`, ⇧⌘F9 = `build.solution`).
- Não remover `build-action` nem `toolbar-build` (contrato de `data-testid`; b21 fica
  registrado na retro como duplicação intencional com escopo explícito).

## Teste primeiro

`tests/e2e/build.spec.ts`: antes de qualquer build, `build-action` contém "Build startup" e
`(Cmd+F9|Ctrl+F9)` e não contém "Rebuild"; depois de um `build-solution`, continua "Build
startup" e o tooltip contém "Build Solution". `zz-acceptance.spec.ts` continua casando
`/Cmd\+F9|Ctrl\+F9/`.

## Aceite

- [ ] Nenhum texto "Rebuild" no Build panel.
- [ ] Escopo startup explícito antes e depois de Build Solution.
- [ ] Atalhos e ações inalterados; testids preservados.
- [ ] `bun run check` verde. Commit `fix(build): label startup build without Rebuild`.
