# 01 — Preflight do release

**Fonte:** `docs/SPEC-release-ci.md` §“Preflight”. **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

`scripts/release-macos.sh --check` reprova, em segundos e sem efeito colateral, qualquer release que
não possa ser publicada com segurança: tag fora do padrão, versão divergente entre as cinco fontes,
tag fora de `main`, notas de release apontando para outra versão, credencial ausente, ferramenta
ausente ou máquina que não é arm64. Uma mensagem por regra, exit 1, nada de assinar nem publicar.

## Pontos de código

- Novo `scripts/release-macos.sh` (POSIX `sh`, `set -eu`), com `ROOT=${RELEASE_ROOT:-<raiz do repo>}`
  como seam de teste e as regras do §“Preflight” do spec.
- Tag: `--tag` > `GITHUB_REF_NAME` > `git describe --tags --exact-match`; aceita `v<semver>` com
  sufixo de pré-release.
- Versões lidas de `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
  `crates/host/Cargo.toml`, `crates/host-http/Cargo.toml` — comparação por igualdade, relatando qual
  arquivo diverge.
- Ancestralidade: `git merge-base --is-ancestor <tag>^{commit} origin/main`; `origin/main` ausente
  vira instrução explícita (`git fetch origin main`).
- Credenciais: exige `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_KEY`,
  `APPLE_API_KEY_ID`, `APPLE_SIGNING_IDENTITY`; `APPLE_API_ISSUER` é opcional. Lista todos os nomes
  ausentes de uma vez, nunca imprime valores.
- Ferramentas: `bun`, `cargo`, `xcrun`, `codesign`, `hdiutil`, `ditto`, `shasum`, `gh`.
- Arquitetura: `uname -m` = `arm64`.
- Novo `scripts/release-selfcheck.sh` (padrão de `scripts/openridr-selfcheck.sh`): roda o preflight
  contra `RELEASE_ROOT` temporário com arquivos plantados e prova cada reprovação isolada + o caso
  aprovado com credenciais fictícias. Não toca em repositório real, keychain ou rede.

## Critérios de aceite

- [ ] `release-macos.sh --check` passa neste worktree com credenciais fictícias e `--tag
      v0.3.0-alpha.11`.
- [ ] Falha com mensagem específica para: tag sem `v`/não-semver, versão divergente em cada uma das
      cinco fontes, tag não ancestral de `origin/main`, heading de `docs/RELEASE-ALPHA.md` diferente,
      cada credencial ausente, ferramenta ausente (`PATH` reduzido), `uname -m` diferente.
- [ ] Nenhuma saída imprime valor de credencial.
- [ ] `sh scripts/release-selfcheck.sh` falha antes do script existir e passa depois; `sh -n` limpo.
