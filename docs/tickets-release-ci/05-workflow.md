# 05 — Workflow do GitHub Actions

**Fonte:** `docs/SPEC-release-ci.md` §“CI” e §“Credenciais”. **Bloqueado por:** 01, 02, 04.
**Status:** ready-for-agent.

## Objetivo

Push de tag `v*` roda o release num runner macOS arm64 e publica a pré-release com o DMG e o
SHA-256. Sem credenciais cadastradas, o job falha em ~30 s listando os nomes ausentes. Com
`workflow_dispatch` e `publish: false`, o Helder valida o caminho inteiro (build, assinatura,
notarização, verificação) sem publicar.

## Pontos de código

- Novo `.github/workflows/release.yml`: gatilhos `push: tags: v*` e `workflow_dispatch` com inputs
  `tag` e `publish` (boolean, default `false`); `permissions: contents: write`;
  `concurrency: group: release-${{ github.ref }}, cancel-in-progress: false`; job único em
  `macos-15`, `timeout-minutes: 90`.
- Passos, na ordem: checkout (`fetch-depth: 0`, `fetch-tags: true`) → `oven-sh/setup-bun` (versão
  fixada) → `release-macos.sh --check` → import do `.p12` em keychain temporário (senha aleatória
  gerada no runner com `openssl rand`, `security create/import/set-key-partition-list`, sem ecoar
  senha) → escrita do `.p8` a partir de `APPLE_API_KEY` base64 em `$RUNNER_TEMP` (`chmod 600`) →
  `bun install --frozen-lockfile` → `bun run check` → `bun test tests/unit` → `release-macos.sh` →
  notas → `gh release create --prerelease --title "OpenRidr <versão>" --notes-file` (só quando
  `publish` não é `false`) → upload do DMG como artifact (14 dias).
- Limpeza em `if: always()`: `security delete-keychain`, remoção do `.p12` e do `.p8`.
- `GITHUB_TOKEN` como `GH_TOKEN` só nos passos que publicam; actions pinadas por SHA.
- Sem E2E, sem `tauri-apps/tauri-action`, sem tag automática.

## Critérios de aceite

- [ ] `actionlint` limpo (baixar o binário oficial em `/tmp`, rodar contra o repo).
- [ ] Os nomes de secret/variable usados no YAML são exatamente os do spec; nenhum valor no arquivo.
- [ ] Passo de preflight aparece antes do import do certificado e do build.
- [ ] `publish: false` no `workflow_dispatch` não chama `gh release create`.
- [ ] Limpeza do keychain tem `if: always()` e cobre falha no meio do build.
- [ ] Workflow não roda E2E nem publica em `pull_request`/`push` de branch.
