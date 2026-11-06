# Evidência — release assinado e notarizado pelo GitHub Actions (issue #44)

Spec: [SPEC-release-ci.md](../SPEC-release-ci.md). Tickets: #58, #61, #63, #65, #66, #68.
Ambiente: macOS 27.2, Apple Silicon (arm64), Command Line Tools (sem Xcode completo),
`bun 1.3.9`, `actionlint 1.7.12`, `shellcheck 0.11.0`, `OpenRidr.app` alpha.11 instalado (Developer
ID) usado só como alvo de leitura da auditoria. Sem nenhuma credencial de assinatura/notarização.

## Executado

### Self-checks dos scripts

`sh scripts/release-selfcheck.sh` → `ok: release self-check passed`. Prova (inclui as regras
acrescentadas na revisão):

- preflight aprova o caso completo (checkout temporário, tag `v0.3.0-alpha.11`) e reprova, cada uma
  com mensagem própria: tag sem `v`, não-semver (`v1.2`), componente com zero à esquerda (`v01.2.3`),
  divergência em cada uma das cinco fontes de versão, heading de `RELEASE-ALPHA.md` de outra versão,
  tag inexistente, tag fora de `main` (commit não mergeado), `origin/main` não buscado, **`HEAD`
  diferente do commit da tag**, heading com versão que só começa com a esperada (`…-extra`), cada
  credencial ausente (com o `.p12` dispensado quando há `NOTARY_KEYCHAIN_PROFILE`), identidade que não
  é Developer ID, ferramenta ausente (`PATH` só com binários plantados, sem `hdiutil`) e `uname -m`
  `x86_64` (com `uname` falso na frente do `PATH`);
- nenhuma saída contém o valor-canário das credenciais;
- `--dmg-only` sobre um `.app` de stub monta um DMG real e confere nome
  (`OpenRidr_9.9.9_aarch64.dmg`), `Applications -> /Applications`, versão do `plutil` e as quatro
  chaves de `$GITHUB_OUTPUT` (`version`, `dmg`, `sha256`, `bytes`); reprova app com versão divergente,
  app sem `Contents/MacOS/openridr` e execução sem versão;
- `--audit` no app instalado aprova 18 Mach-O (17 com a identidade do Helder, 1 `createdump` da
  Microsoft) e reprova identidade trocada nomeando o arquivo;
- a auditoria reconhece os oito magic de Mach-O sintéticos (thin 32/64 bits nas duas endiannesses,
  fat32, fat64 e as duas variantes de campos trocados) e ignora `Java.class` (mesmo `cafebabe`, com
  `nfat_arch` = versão 61) e um arquivo de dados;
- o caminho assinado, com `xcrun`/`codesign`/`spctl` falsos, chega à notarização e deixa
  `RUNNER_TEMP` limpo **no sucesso, na rejeição e quando a chave é inválida** (sem `AuthKey_*`, sem
  `notary-key.*` e sem zip de transporte); um `openssl` falso registra, no instante da decodificação,
  que o arquivo já está no caminho determinístico `notary-key.<pid>/AuthKey_<id>.p8` com modo 0600 e o
  diretório com 0700;
- a auditoria falha com caminho contendo quebra de linha, falha com symlink que aponta para fora do
  `.app` e aceita symlink interno (o alvo regular é o auditado); arquivo de 4 bytes com magic de
  Mach-O é ignorado (`tiny4`), não auditado;
- a auditoria reprova binário com assinatura quebrada (`does not verify with codesign --verify
  --strict`), não só metadado ausente;
- modo completo sobre um bundle ad-hoc para na auditoria (`has no hardened runtime`) sem criar
  `AuthKey_*.p8` nem sobras em `RUNNER_TEMP`;
- `--dry-run` com identidade no ambiente e um `bun` de mentira no `PATH` registra que o build recebeu
  `APPLE_SIGNING_IDENTITY` **vazio** (ensaio ad-hoc) e ainda assim monta e verifica o DMG.

`sh scripts/release-notes-selfcheck.sh` → `ok: release-notes self-check passed`. Prova as notas +
bloco `## Verificação` (DMG, bytes, hash, linha compatível com `shasum`, comando de conferência),
os resultados opcionais de `codesign`/`spctl`/`stapler` e cada rejeição (heading de outra versão,
hash não-hex, bytes zero ou decimal, arquivo ausente, notas com SHA-256 escrito à mão, uso inválido).

### Os testes não são vazios (mutação)

- Apagar a chamada `check_versions` do preflight: `release-selfcheck.sh` falha em
  `expected failure: --check --tag v0.3.0-alpha.11`.
- Neutralizar a validação de SHA-256 das notas: `release-notes-selfcheck.sh` falha em
  `expected failure: 0.3.0-alpha.11 abc 12345 …`.
- Remover a comparação `HEAD == tag`: self-check falha em `expected failure: --check --tag …`.
- Voltar o heading para casamento por prefixo: falha em `expected failure: --check --tag …`.
- Tirar a limpeza da identidade no `--dry-run`: falha em
  `the dry run passed APPLE_SIGNING_IDENTITY='…' to the build`.
- Remover o `codesign --verify --strict` por arquivo: falha em
  `broken signature message is not specific: … has no hardened runtime`.
- Voltar a chave para `mktemp` dentro de `$(...)`: falha em
  `the key was not at the deterministic path with 0600 while being written`.
- Remover `check_bundle_tree`: falha em `a symlink leaving the bundle was accepted`.
- Remover a guarda de 8 bytes: falha em `the audit treated tiny4 as Mach-O`.
- Remover a chamada de `prepare_notary_key` no shell principal (deixando a chave nascer dentro do
  `$(...)` do `notarytool`): o self-check falha em
  `after a successful notarization these stayed behind: AuthKey_SELFTEST123…`.
- Tirar `cafebabf` da lista de magics: falha em `the audit ignored fat64 (fat64 included)`.
- Aceitar `cafebabe` sem desambiguar o `nfat_arch`: falha em
  `the audit treated Java.class as Mach-O`.
- Exigir `.p12` mesmo com `NOTARY_KEYCHAIN_PROFILE`: falha em
  `expected success: --check --tag v0.3.0-alpha.11`.

### Preflight no repositório real

```text
# na branch (HEAD != commit da tag): recusa
$ … sh scripts/release-macos.sh --check --tag v0.3.0-alpha.11
release: HEAD is 424a59d… but tag v0.3.0-alpha.11 points at 196e3b3…; check out the tag
before releasing                                                                   (exit 1)

# worktree destacada no commit da tag: aprova
$ git worktree add -q --detach /tmp/relcheck v0.3.0-alpha.11
$ APPLE_CERTIFICATE=… APPLE_CERTIFICATE_PASSWORD=… APPLE_API_KEY=… APPLE_API_KEY_ID=… \
  APPLE_SIGNING_IDENTITY='Developer ID Application: Helder Santos (K6M85697D4)' \
  RELEASE_ROOT=/tmp/relcheck sh scripts/release-macos.sh --check --tag v0.3.0-alpha.11
release: preflight ok: v0.3.0-alpha.11 (0.3.0-alpha.11), architecture arm64        (exit 0)

# sem credenciais
$ sh scripts/release-macos.sh --check --tag v0.3.0-alpha.11
release: missing credentials: APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_KEY(or
NOTARY_KEYCHAIN_PROFILE) APPLE_SIGNING_IDENTITY (names only; …)                    (exit 1)

# caminho manual: NOTARY_KEYCHAIN_PROFILE dispensa o .p12
$ unset APPLE_API_KEY APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
$ NOTARY_KEYCHAIN_PROFILE=notary … sh scripts/release-macos.sh --check …            (exit 0)
```

### Ensaio do caminho completo sem credenciais

`sh scripts/release-macos.sh --dry-run --tag v0.3.0-alpha.11` buildou o app
(`Signing with identity "-"`, o build local ad-hoc do Tauri), montou o DMG com `hdiutil`, montou-o em
ponto temporário e conferiu `OpenRidr.app`, o symlink `Applications`, `Mach-O 64-bit executable
arm64` e a versão. Saída: `dmg=<tmp>/OpenRidr_0.3.0-alpha.11_aarch64.dmg`, `sha256=…`, `bytes=…`.

Cinco montagens do mesmo app ad-hoc deram tamanhos `69.516.096`, `69.516.278`, `69.516.000`,
`69.516.228` e `69.516.272` bytes, com hashes diferentes: o DMG **não** é reproduzível byte a byte,
por isso o SHA-256 publicado sempre vem do run.

Nenhuma assinatura nem notarização foi tentada no ensaio: com wrappers de `xcrun` e `codesign` na
frente do `PATH` que registrariam qualquer chamada (e sairiam com 1), o ensaio terminou com exit 0 e
o log dos wrappers ficou **vazio** (`RELEASE_SKIP_BUILD=1` para não envolver o build do Tauri).

### Auditoria de Mach-O

```text
$ sh scripts/release-macos.sh --audit /Applications/OpenRidr.app
release: audit ok: 18 Mach-O (17 with 'Developer ID Application: Helder Santos (K6M85697D4)',
1 createdump from Microsoft)                                                       (exit 0)

$ APPLE_SIGNING_IDENTITY='Developer ID Application: Outra Pessoa (ZZZZZZZZZZ)' \
  sh scripts/release-macos.sh --audit /Applications/OpenRidr.app
release: Contents/MacOS/openridr is not signed with 'Developer ID Application: Outra
Pessoa (ZZZZZZZZZZ)'                                                                (exit 1)
```

Os 18: `Contents/MacOS/openridr`, `Contents/Resources/dotnet/dotnet`,
`host/fxr/10.0.14/libhostfxr.dylib`, 13 dylibs + `createdump` em
`shared/Microsoft.NETCore.App/10.0.14/`, `Resources/netcoredbg/{libdbgshim.dylib,netcoredbg}`.
O csharp-ls é só gerenciado (nenhum Mach-O). Dylibs não trazem `TeamIdentifier` no
`codesign -dv --verbose=4`; por isso a auditoria compara `Authority`.

### Lint

- `actionlint 1.7.12` (com `shellcheck 0.11.0` no `PATH`) em `.github/workflows/release.yml`:
  exit 0, nenhuma mensagem.
- `shellcheck -s sh -S warning` nos quatro scripts novos: exit 0 com
  `-e SC1007,SC2100` (o padrão `CDPATH= cd` já usado no repo e dois falsos positivos de aritmética).

### Bateria

- `bun run check` (tsc + `cargo check --workspace --all-targets`): passa no hook de cada commit.
- `bun run test:unit`: 35 passaram, 0 falharam.
- `cargo test --workspace` com `PATH=$HOME/.dotnet:…` e `DOTNET_ROOT=$HOME/.dotnet`: 286 + 11 + 14 +
  98 + 10 + 7 + 1 passaram, 0 falharam.
  Sem `$HOME/.dotnet` no `PATH`, `run_configs::tests::launch_profile_runs_fixture_with_effective_environment_and_arguments`
  falha por não achar o SDK 8.0.408 do `global.json` — é ambiente, não o diff: o teste não toca este
  trabalho e passa com o `PATH` documentado no `AGENTS.md`.
- `bun run test:e2e` **não** rodado: este trabalho não muda UI/host e a suíte tem falhas conhecidas no
  HEAD.

## Revisão

`/code-review` por delegação (Pi, `openai-codex/gpt-6.1-terra`, thinking medium) sobre
`git merge-base origin/main HEAD`: 8 achados de padrão + 3 de spec, 3 altos. Os 10 itens encaminhados
foram corrigidos com teste que falha antes e passa depois (seção “Os testes não são vazios”).

Re-check dos altos: o item 2 (checkout/HEAD da tag) passou; os itens 1 e 3 não, e voltaram corrigidos —
`notarize` chamava `prepare_notary_key` apenas dentro do `$(...)` do `notarytool` (chave órfã) e a
detecção dependia de `file`, que responde `data` para fat64. Agora a chave é criada no shell dono do
`trap` e a detecção lê os quatro primeiros bytes, com `cafebabe` desambiguado do `.class` do Java.

O workflow mudou de checkout, `concurrency`, ordem de upload e limpeza do keychain sem execução
remota — segue não verificado abaixo.

## Não verificado

- Assinatura Developer ID, notarização aceita pela Apple, `stapler` e publicação da pré-release:
  exigem `APPLE_CERTIFICATE`/`APPLE_API_KEY` cadastrados e uma tag. Passo a passo do primeiro run em
  [release-signing.md](../release-signing.md).
- O workflow rodando no GitHub Actions (mesmo motivo): validado só por `actionlint`.
- `notarytool` com `--issuer` real (Team API Key) e o caminho de rejeição com `notarytool log` real.
- Republicação de uma tag já publicada (`gh release edit` + `upload --clobber`).
- macOS Intel e Windows.
- `hdiutil create` emite aviso de deprecação no macOS 27 ("use `diskutil image create from`"); o DMG
  é criado e verificado normalmente, e o passo continua o mesmo do processo manual documentado.
