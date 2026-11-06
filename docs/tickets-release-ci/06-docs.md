# 06 — Docs do release CI

**Fonte:** `docs/SPEC-release-ci.md` §“Credenciais” e §“Notas e publicação”. **Bloqueado por:** 05.
**Status:** ready-for-agent.

## Objetivo

Quem for publicar entende o caminho novo sem ler o script: o que o push da tag faz, quais nomes de
secret cadastrar (e como gerar cada um), o que o preflight recusa, como fazer o primeiro run real
com `workflow_dispatch` e o que fazer quando a Apple rejeita. O `RELEASE-ALPHA.md` deixa de ter
SHA-256 escrito à mão.

## Pontos de código

- `docs/release-signing.md`: nova seção de CI (workflow, secrets, variável de identidade,
  `workflow_dispatch` com `publish: false`, primeiro run real, leitura do log de rejeição),
  passo-a-passo de cadastro (criar Team API Key no App Store Connect, exportar o `.p12` do
  certificado, `base64` de cada um, onde colar), e o caminho manual passa a ser “rode
  `scripts/release-macos.sh`”, mantendo o “por quê” (hardened runtime, entitlements do runtime
  embutido, `hdiutil` em vez de `--bundles dmg`, sem hash em runtime).
- `docs/decisions.md`: entrada “Release CI” com as decisões do grill (script único, API key, versão
  pela tag, publicação direta, fail-fast, sem tauri-action, auditoria de Mach-O com a exceção
  `createdump`, sem `RELEASE-CI.md`).
- `docs/RELEASE-ALPHA.md`: remover o bloco com SHA-256/tamanho escritos à mão e apontar que a
  verificação é gerada pelo CI.
- `docs/README.md`: linha do índice para spec/tickets/retro/evidência; `docs/MAPA.md` se o comando
  novo merecer ponteiro.
- `docs/evidence/release-ci.md`: o que foi executado (self-checks, actionlint, `--dry-run`), com
  caminhos normalizados, e o que ficou não verificado.
- `docs/RETRO-release-ci.md` só no fim do fluxo, fora deste ticket.

## Critérios de aceite

- [ ] `docs/release-signing.md` ensina o cadastro dos 5 secrets + 1 variable e o primeiro run sem
      pedir para ler o YAML.
- [ ] Nenhum segredo, valor de credencial ou caminho pessoal nos docs (checar com
      `grep -n "/Users/\|APPLE_API_KEY=" docs/release-signing.md`).
- [ ] Índice de `docs/README.md` lista a feature; `RELEASE-ALPHA.md` sem hash manual.
- [ ] `docs/evidence/release-ci.md` separa “executado” de “não verificado”.
