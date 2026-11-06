# 02 — Corpo das notas de release

**Fonte:** `docs/SPEC-release-ci.md` §“Notas e publicação”. **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

O corpo publicado na pré-release passa a ser gerado: as notas humanas de `docs/RELEASE-ALPHA.md`
mais um bloco `## Verificação` com SHA-256, tamanho e os resultados de `codesign`/`spctl`/`stapler`
do run. A pessoa que baixa lê o hash certo sem ninguém copiar à mão.

## Pontos de código

- Novo `scripts/release-notes.sh <versão> <sha256> <bytes> [notas]` (POSIX `sh`): imprime
  `docs/RELEASE-ALPHA.md` (ou o arquivo passado) seguido do bloco gerado, com linhas de comando de
  conferência (`shasum -a 256`) e uma linha por verificação informada por env
  (`RELEASE_VERIFY_CODESIGN`, `RELEASE_VERIFY_SPCTL`, `RELEASE_VERIFY_STAPLER`), omitidas quando
  vazias.
- Recusa rodar sem versão/hash/tamanho numéricos, e recusa se o primeiro heading das notas não for
  `# OpenRidr <versão>` (mesma regra do preflight, aqui para não publicar notas de outra versão).
- Novo `scripts/release-notes-selfcheck.sh`: falha antes, passa depois; confere que o corpo contém
  as notas, o hash certo, os bytes e que **não** há SHA-256 escrito à mão no `RELEASE-ALPHA.md`.

## Critérios de aceite

- [ ] `sh scripts/release-notes.sh 0.3.0-alpha.11 <hash> 12345` imprime notas + bloco de verificação
      com hash, bytes e as linhas de verificação informadas.
- [ ] Versão divergente do heading ou argumento ausente → exit 1 com mensagem.
- [ ] `sh scripts/release-notes-selfcheck.sh` falha antes e passa depois; `sh -n` limpo.
