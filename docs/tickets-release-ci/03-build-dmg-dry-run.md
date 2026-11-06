# 03 — Build, DMG e ensaio sem credenciais

**Fonte:** `docs/SPEC-release-ci.md` §“Fonte única de verdade” e §“Verificação do artefato”.
**Bloqueado por:** 01. **Status:** ready-for-agent.

## Objetivo

O script monta o DMG do jeito que a release usa (`hdiutil`, `.app` já grampeado dentro, symlink para
`/Applications`, UDZO) e ensaia o caminho inteiro sem credenciais: `--dry-run` builda, monta o DMG,
confere o layout e imprime exatamente os comandos de assinatura/notarização que seriam executados,
terminando com o SHA-256 do DMG não assinado. É a prova possível antes de existir Developer ID no
runner. O DMG não é publicado.

## Pontos de código

- `scripts/release-macos.sh`: `--dry-run` (preflight relaxado: credenciais não exigidas, resto igual)
  e `--dmg-only` (monta/confere DMG a partir de um `.app` existente, sem build; usado pelo ensaio e
  pelo self-check).
- Build: `bun install --frozen-lockfile` quando `node_modules` ausente, depois
  `APPLE_SIGNING_IDENTITY` só quando existir e `bun run tauri build --target aarch64-apple-darwin
  --bundles app`. Caminho do `.app`: `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/OpenRidr.app`,
  com fallback para `src-tauri/target/release/...` (é onde o build local sem `--target` cai).
  `RELEASE_SKIP_BUILD=1` e `RELEASE_APP` pulam/definem o app.
- DMG em `$RUNNER_TEMP` (default `${TMPDIR:-/tmp}`), nome `OpenRidr_<versão>_aarch64.dmg`; estágio em
  diretório temporário próprio, removido no fim.
- Conferência do DMG: monta em ponto temporário, exige `OpenRidr.app`, exige o symlink
  `Applications -> /Applications`, `file` arm64 nos binários, versão do `defaults read` igual à tag,
  desmonta sempre (inclusive em erro), `shasum -a 256` e `stat` do tamanho.
- Saída em `$GITHUB_OUTPUT` quando definido: `version`, `dmg`, `sha256`, `bytes`.
- `scripts/release-selfcheck.sh`: monta um `.app` de stub (binário `sh` copiado, `Info.plist` mínimo)
  e prova `--dmg-only` (layout, symlink, sha256, saída `GITHUB_OUTPUT`), inclusive falhando quando o
  stub não tem o binário esperado.

## Critérios de aceite

- [ ] `sh scripts/release-macos.sh --dmg-only --app <stub>` monta e verifica um DMG real; imprime
      `dmg=`, `sha256=`, `bytes=`; escreve as quatro chaves em `$GITHUB_OUTPUT`.
- [ ] App sem `Contents/MacOS/openridr` → exit 1 com mensagem.
- [ ] `--dry-run` neste worktree imprime os comandos de `codesign`/`notarytool` planejados e o
      SHA-256, sem chamar `codesign --sign` com Developer ID nem `notarytool`.
- [ ] Falha antes (self-check sem o script) e passa depois; `sh -n` limpo.
