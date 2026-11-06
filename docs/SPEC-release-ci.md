# SPEC — Release assinado e notarizado pelo GitHub Actions

Fonte: issue #44 (`Heldinhow/openridr`). Decisões de grill: `docs/decisions.md` (2026-11-06, “Release
CI”). Tickets: issues #58 (preflight), #61 (notas), #63 (build/DMG/ensaio), #65 (assinatura e
notarização), #66 (workflow), #68 (docs) — todas `Part of #44`.

## Problema e solução

Hoje cada alfa é construída, assinada (Developer ID, hardened runtime, Mach-O embutidos), notarizada,
grampeada e publicada à mão seguindo `docs/RELEASE-ALPHA.md` / `docs/release-signing.md`: ~40 min de
passos copiados, com dois pontos de falha silenciosa (um Mach-O embutido sem a nossa assinatura e o
`stapler` pendente no DMG).

A solução: o **processo vira script** (`scripts/release-macos.sh`) e o push de uma tag `v*` roda esse
mesmo script num runner macOS arm64, publicando a pré-release com o DMG e o SHA-256. O conteúdo
entregue pelo CI é o mesmo da execução manual porque é literalmente o mesmo código — não uma
reimplementação em YAML.

## Contrato do release

1. Push de uma tag `v<versão>` (ex.: `v0.3.0-alpha.12`) produz uma pré-release `OpenRidr <versão>`
   com um único anexo `OpenRidr_<versão>_aarch64.dmg` e o SHA-256 no corpo.
2. A tag é a identidade da release. O preflight falha em segundos se a tag não casar exatamente com
   `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e
   `crates/{host,host-http}/Cargo.toml`; o bump continua sendo um commit `chore(release)` antes da
   tag, como nas alphas 1–11.
3. A tag precisa apontar para um commit ancestral de `origin/main`. Release de galho solto falha no
   preflight.
4. Nada é publicado sem notarização aceita e sem `stapler` validado no `.app` **e** no DMG. Falha em
   qualquer etapa aborta o job; não existe caminho que publique artefato parcial.
5. Faltando qualquer credencial, o job falha no preflight (~30 s), antes do build e sem assinar nada,
   listando os nomes ausentes. Nunca há publicação de artefato não assinado.
6. O mesmo script roda na máquina do Helder (`docs/release-signing.md`), com o keychain `notary` ou
   com a API key, produzindo o mesmo DMG. Não há dois processos.

## Fonte única de verdade: `scripts/release-macos.sh`

Um script, três modos, tudo por ambiente:

- `--check`: preflight, sem efeito colateral (~30 s).
- `--dry-run`: preflight relaxado (credenciais não exigidas), build, montagem e verificação do DMG,
  impressão dos comandos de assinatura/notarização que seriam executados, SHA-256 do DMG não
  assinado. É a prova possível sem credenciais.
- sem flag: caminho completo — build com Developer ID → auditoria de Mach-O → notarizar/grampear
  `.app` → montar DMG → assinar → notarizar/grampear → verificação completa → SHA-256.

Saída: linhas humanas sempre e, quando `$GITHUB_OUTPUT` existe, `version`, `dmg`, `sha256`, `bytes`.

Entradas: `APPLE_SIGNING_IDENTITY` (variable do repo), `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_KEY` (base64 do `.p8`), `APPLE_API_KEY_ID`,
`APPLE_API_ISSUER` (opcional: só Team API Key), `--tag` (default `GITHUB_REF_NAME` ou
`git describe --tags --exact-match`). `RELEASE_ROOT` existe só como seam de teste.

Temporários em `$RUNNER_TEMP` (estágio do DMG, `.app` zipado). Nada em `~/Downloads`, nada em pasta
sincronizada.

## Preflight (`--check`)

Falha com uma mensagem por regra e exit 1:

1. tag `v<semver>` (aceita sufixo de pré-release);
2. versão idêntica nas cinco fontes;
3. commit da tag ancestral de `origin/main`;
4. primeiro heading de `docs/RELEASE-ALPHA.md` é `# OpenRidr <versão>`;
5. as credenciais presentes — só nomes, nunca valores;
6. ferramentas no PATH: `bun`, `cargo`, `xcrun`, `codesign`, `hdiutil`, `ditto`, `shasum`, `gh`;
7. `uname -m` = `arm64`.

## Assinatura e auditoria de Mach-O

O build usa `APPLE_SIGNING_IDENTITY` (o `signingIdentity: "-"` do config fica para o build local) e
`beforeBuildCommand` roda `vendor-netcoredbg.sh` + `vendor-analysis.sh`, que assinam o runtime .NET
privado (muxer com `dotnet.entitlements`, dylibs sem entitlements) e o netcoredbg com a mesma
identidade. Depois do bundle, a auditoria percorre **todos** os Mach-O do `.app` (hoje 18: 1 binário
principal, 15 no runtime .NET, 2 no netcoredbg) e exige, em cada um:

- `codesign --verify --strict` ok;
- `flags=0x10000(runtime)`;
- `Timestamp=` presente;
- `Authority=Developer ID Application: <nossa identidade>`.

Exceção única e nominal: `createdump`, assinado pela Microsoft
(`Developer ID Application: Microsoft Corporation (UBF8T346G9)`), que o runtime oficial traz assim e
o `vendor-analysis.sh` não reassina. Qualquer outro `Authority` estranho é falha — foi o risco da
alpha.9. Dylibs não têm `TeamIdentifier`, então a comparação é por `Authority`, não por time.

## Notarização e grampo

`xcrun notarytool submit <arquivo> --key <p8> --key-id <id> [--issuer <issuer>] --wait
--output-format json --no-progress`, sem retry automático. Rejeição baixa e imprime
`notarytool log <id>` e falha. Ordem: `.app` zipado (`ditto -c -k --keepParent --sequesterRsrc`) →
`stapler staple`/`validate` no `.app` → DMG montado com `hdiutil` (não `--bundles dmg`, que refaz o
app e descarta o ticket) → `codesign` no DMG → notarizar/grampear/validar o DMG.

## Verificação do artefato

Exatamente a lista de `docs/release-signing.md`: `codesign --verify --deep --strict` no app e no app
montado, `Authority`/`TeamIdentifier`/`flags` do app, `spctl --assess` no app e no DMG,
`stapler validate` nos dois, `file` = Mach-O arm64, `defaults read` da versão, `shasum -a 256`.
Quarentena e translocação ficam fora: são comportamento do macOS na máquina de quem baixa.

## Notas e publicação

`scripts/release-notes.sh <versão> <sha256> <bytes>` imprime o corpo: conteúdo atual de
`docs/RELEASE-ALPHA.md` (que deixa de ter SHA-256 escrito à mão) + bloco `## Verificação` gerado com
hash, bytes e os resultados de `codesign`/`spctl`/`stapler`. O workflow usa `gh release create
--prerelease --notes-file`. Pré-release sempre que a versão tem `-`.

## CI

`.github/workflows/release.yml`, um job em `macos-15` (arm64), `timeout-minutes: 90`,
`permissions: contents: write`, `concurrency: release` sem cancelar em andamento:

checkout (`fetch-depth: 0`) → `setup-bun` → `release-macos.sh --check` → import do `.p12` em keychain
temporário (senha aleatória do runner, removido em `if: always()`) → escrita do `.p8` → `bun install
--frozen-lockfile` → `bun run check` + `bun test tests/unit` → `release-macos.sh` → notas →
`gh release create` → upload do DMG como artifact (14 dias).

`workflow_dispatch` aceita `tag` e `publish` (default `false`): roda build, assinatura, notarização e
verificação e **para antes de publicar**. Não roda E2E (falhas conhecidas do HEAD) nem em
`macos-14`/Intel/Windows. Actions pinadas por SHA; sem `tauri-apps/tauri-action` (ele faz bundle e
notarização próprios e divergiria do processo manual).

## Credenciais

Só nomes no repositório e no doc; valores só no GitHub (secrets) e no keychain do Helder. O
passo-a-passo de cadastro e do primeiro run real fica em `docs/release-signing.md`. API key do App
Store Connect é Team API Key (issuer obrigatório) ou Individual (sem issuer); o script passa
`--issuer` só quando `APPLE_API_ISSUER` existe.

## Testes

O que dá para provar sem credenciais, com teste que falha antes e passa depois:

- `scripts/release-selfcheck.sh`: preflight reprovando cada regra isoladamente (tag inválida, versão
  divergente, heading errado, credencial ausente) e aprovando o caso completo com credenciais
  fictícias; montagem do DMG a partir de um `.app` de stub, com layout, symlink e SHA-256
  conferidos; auditoria de Mach-O rodada contra `/Applications/OpenRidr.app` (leitura apenas), que
  deve aprovar a última release real e reprovar uma identidade trocada;
- `scripts/release-notes-selfcheck.sh`: corpo contém notas + bloco gerado, sem hash manual;
- `actionlint` no workflow.

Não verificado até o primeiro tag com credenciais: assinatura Developer ID real, notarização
aceita, `stapler`, publicação e o efeito do artefato baixado. Isso fica registrado como não
verificado, com o passo-a-passo do primeiro run.

## Fora de escopo

- macOS Intel, Windows, Linux (não executados, não suportados).
- Build universal/`--target universal-apple-darwin`.
- Assinatura de `.dmg` dentro do bundler do Tauri (`--bundles dmg`).
- Tag automática, bump automático de versão, changelog automático a partir de commits.
- E2E no workflow de release.
- Updater/`TAURI_SIGNING_PRIVATE_KEY`.
- Segredos em arquivo do repositório, `git stash`, `--no-verify`, push ou PR pela sessão de agente.
