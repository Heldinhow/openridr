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
- `--dry-run`: preflight relaxado (credenciais não exigidas), build **ad-hoc** (a identidade do
  ambiente é limpa para o ensaio nunca assinar de verdade), montagem e verificação do DMG, impressão
  dos comandos de assinatura/notarização que seriam executados, SHA-256 do DMG não assinado. É a
  prova possível sem credenciais.
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
2. a tag existe e `HEAD` é exatamente o commit dela — buildar a ponta de um branch e publicar sob a
   tag é o erro que isso impede, e o workflow faz checkout da própria tag;
3. versão idêntica nas cinco fontes;
4. commit da tag ancestral de `origin/main`;
5. primeiro heading de `docs/RELEASE-ALPHA.md` tem a versão exata (não por prefixo: `0.3.0` não
   aceita as notas de `0.3.0-alpha.11`);
6. as credenciais presentes — só nomes, nunca valores;
7. ferramentas no PATH: `bun`, `cargo`, `xcrun`, `codesign`, `hdiutil`, `ditto`, `shasum`, `gh`;
8. `uname -m` = `arm64`.

## Assinatura e auditoria de Mach-O

O build usa `APPLE_SIGNING_IDENTITY` (o `signingIdentity: "-"` do config fica para o build local) e
`beforeBuildCommand` roda `vendor-netcoredbg.sh` + `vendor-analysis.sh`, que assinam o runtime .NET
privado (muxer com `dotnet.entitlements`, dylibs sem entitlements) e o netcoredbg com a mesma
identidade. Depois do bundle, a auditoria lista **todos** os Mach-O do `.app` pelos quatro primeiros bytes
(thin 32/64 bits nas duas endiannesses e os contêineres fat/fat64, inclusive com campos trocados;
`cafebabe` é desambiguado do `.class` do Java pelo `nfat_arch`, que num binário universal é pequeno e
num class file é a versão 45+); arquivo com menos de 8 bytes não é candidato, caminho com quebra de
linha faz a auditoria falhar (a varredura é por linha) e symlink que aponta para fora do `.app` também
(links internos são cobertos pelo alvo regular) — hoje 18:
1 binário principal, 15 no runtime .NET, 2 no netcoredbg — e exige, em cada um:

- `codesign --verify --strict` ok (a assinatura é verificada criptograficamente por arquivo, não só
  lida como metadado);
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
`permissions: contents: write`, `concurrency: release` — grupo único no repositório, sem cancelar em
andamento (um dispatch e um push da mesma tag não podem disputar a release):

checkout da própria tag (`ref`, `fetch-depth: 0`) → `setup-bun` → `release-macos.sh --check` → import
do `.p12` em keychain temporário (senha aleatória do runner; o caminho é registrado **antes** da
criação, para o passo `if: always()` sempre poder apagá-lo) → escrita do `.p8` → `bun install
--frozen-lockfile` → `bun run check` + `bun test tests/unit` → `release-macos.sh` → notas →
`gh release create --prerelease` → upload do DMG como artifact (14 dias). Republicar uma tag existente
sobe o DMG (`--clobber`) **antes** de editar o corpo: o texto anuncia um hash, nunca pode conviver com
o anexo anterior.

`workflow_dispatch` aceita `tag` e `publish` (default `false`): roda build, assinatura, notarização e
verificação e **para antes de publicar**. Não roda E2E (falhas conhecidas do HEAD) nem em
`macos-14`/Intel/Windows. Actions pinadas por SHA; sem `tauri-apps/tauri-action` (ele faz bundle e
notarização próprios e divergiria do processo manual).

## Credenciais

Só nomes no repositório e no doc; valores só no GitHub (secrets) e no keychain do Helder. O
passo-a-passo de cadastro e do primeiro run real fica em `docs/release-signing.md`. API key do App
Store Connect é Team API Key (issuer obrigatório) ou Individual (sem issuer); o script passa
`--issuer` só quando `APPLE_API_ISSUER` existe.

O `.p8` é escrito em `$RUNNER_TEMP` num diretório privado (`notary-key.<pid>`, 0700, criado com
`umask 077`) e o caminho é definido **antes** de o arquivo existir, para o `trap EXIT` conhecê-lo desde
o primeiro instante; o arquivo é criado no shell principal (0600 desde o primeiro byte), antes de
qualquer captura de saída do `notarytool` — nunca dentro de uma substituição de comando. O self-check roda o caminho assinado com
`notarytool` falso e prova que o arquivo não fica, nem no sucesso nem na rejeição.
`APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD` só são exigidos no caminho do CI (que importa o
`.p12`); numa máquina que usa `NOTARY_KEYCHAIN_PROFILE` o certificado já está no keychain e o script
não pede os `.p12`.

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
