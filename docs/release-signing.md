# Assinar, notarizar e publicar o DMG (macOS ARM64)

Vale só para o build de release. O build local (`bun run app:install`) continua ad-hoc e sem
notarização — ver [install-app.md](install-app.md).

Um script faz o caminho inteiro, e o workflow do GitHub Actions chama o mesmo script: o artefato do
CI é o artefato de uma execução manual, não uma reimplementação em YAML.

```sh
sh scripts/release-macos.sh --check --tag v0.3.0-alpha.12   # preflight, ~30 s, sem efeito
sh scripts/release-macos.sh --tag v0.3.0-alpha.12           # build, assina, notariza, grampeia
```

Nada de segredo entra no repositório: a identidade é pública, o `.p12` e a API key vivem nos
GitHub Secrets e, na máquina do Helder, no keychain (`notary` do `notarytool`). O certificado nunca
é exportado para o repo.

## O que o push de uma tag faz

`.github/workflows/release.yml`, job único em `macos-15` (arm64), disparado por tag `v*`:

1. **Preflight** (`release-macos.sh --check`): tag `v<semver>`, `HEAD` igual ao commit da tag (o
   workflow faz checkout da própria tag, não da ponta do branch), versão idêntica em `package.json`,
   `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `crates/host/Cargo.toml` e
   `crates/host-http/Cargo.toml`, tag ancestral de `origin/main`, primeiro heading de
   `docs/RELEASE-ALPHA.md` com a versão exata, credenciais presentes, ferramentas no `PATH` e
   `uname -m` = `arm64`. Falha aqui custa ~30 s e deixa de gastar ~40 min de runner.
2. Importa o `.p12` num keychain temporário (senha aleatória do runner) e escreve o `.p8` da API key
   em `$RUNNER_TEMP` (`chmod 600`).
3. `bun install --frozen-lockfile`, `bun run check`, `bun run test:unit`. Sem E2E (as falhas
   conhecidas do HEAD não bloqueiam release).
4. `release-macos.sh`: build com Developer ID, auditoria de todos os Mach-O, notarização e grampo do
   `.app`, DMG montado com `hdiutil`, DMG assinado, notarizado e grampeado, verificação completa.
5. `release-notes.sh` gera o corpo (notas + bloco `## Verificação` com SHA-256, bytes e resultado de
   `codesign`/`spctl`/`stapler`) e `gh release create --prerelease` publica o DMG.
6. O DMG também fica como artifact do run por 14 dias (`OpenRidr-<versão>`), para republicar sem
   rebuild. O certificado e a API key são removidos do runner em `if: always()`.

Faltando qualquer credencial, o run falha no preflight listando os nomes ausentes e **nada é
publicado**: não existe caminho que publique artefato não assinado.

`workflow_dispatch` (manual) aceita `tag` e `publish` (default `false`): roda os passos 1–4 e para
antes de publicar. É o ensaio que usa as credenciais reais sem criar tag nem release.

## Credenciais a cadastrar (uma vez, só o Helder)

Settings → Secrets and variables → Actions, no repositório `Heldinhow/openridr`:

| Nome | Tipo | Conteúdo |
|---|---|---|
| `APPLE_SIGNING_IDENTITY` | **variable** | `Developer ID Application: Helder Santos (K6M85697D4)` — nome público, igual ao do `security find-identity` |
| `APPLE_CERTIFICATE` | secret | base64 do `.p12` do certificado **Developer ID Application** com a chave privada |
| `APPLE_CERTIFICATE_PASSWORD` | secret | senha escolhida na exportação do `.p12` |
| `APPLE_API_KEY` | secret | base64 do arquivo `AuthKey_<KEYID>.p8` da API key do App Store Connect |
| `APPLE_API_KEY_ID` | secret | o Key ID da API key (10+ caracteres alfanuméricos) |
| `APPLE_API_ISSUER` | secret | Issuer ID (UUID) — obrigatório para **Team** API Key; deixe vazio para Individual |

Passo a passo:

1. **API key.** App Store Connect → *Users and Access* → *Integrations* (Keys) → *Team Keys* →
   gerar uma key com papel **Developer** → baixar o `.p8` (o download acontece uma única vez). Anote
   o **Key ID** e o **Issuer ID** mostrado acima da lista de keys.
   `base64 -i AuthKey_<KEYID>.p8 | pbcopy` → cole em `APPLE_API_KEY`; Key ID em `APPLE_API_KEY_ID`;
   Issuer ID em `APPLE_API_ISSUER`.
2. **Certificado.** Keychain Access → *Meus Certificados* → `Developer ID Application: Helder
   Santos (K6M85697D4)` → *Exportar…* → `.p12` com senha → `base64 -i DeveloperID.p12 | pbcopy` →
   cole em `APPLE_CERTIFICATE`; a senha vai em `APPLE_CERTIFICATE_PASSWORD`.
3. **Variável.** Settings → Variables → nova `APPLE_SIGNING_IDENTITY` com o nome exato acima.
   O script recusa identidade que não comece com `Developer ID Application: `.
4. **Conferência.** `gh secret list --repo Heldinhow/openridr` e `gh variable list --repo
   Heldinhow/openridr` mostram os cinco secrets e a variável (nomes; valores não são legíveis).

Nunca leia, copie ou imprima os valores no chat ou em arquivo do repo.

## Primeiro run real (depois do cadastro)

1. Ensaio sem publicar, na tag existente ou numa tag nova de teste:
   `gh workflow run release.yml --repo Heldinhow/openridr -f tag=v0.3.0-alpha.12 -f publish=false`.
   O run faz build, assinatura, notarização e verificação; para antes do `gh release create`.
2. Confira no log: `audit ok: <n> Mach-O`, `notarization accepted: OpenRidr.zip`,
   `stapled and validated: …/OpenRidr.app`, `notarization accepted: OpenRidr_<versão>_aarch64.dmg`,
   `artifact verified: codesign, spctl and stapler`.
3. Prepare a versão: `chore(release): prepare <versão>` (as cinco fontes + `docs/RELEASE-ALPHA.md`),
   merge no `main`, `git tag v<versão>` e `git push origin v<versão>`. O push da tag é o ato de
   aprovação da publicação.
4. Se a Apple **rejeitar**, o job imprime `notarytool log <id>` e falha sem publicar. Corrija o item
   apontado (assinatura, hardened runtime, entitlements, binário aninhado), refaça o build; nunca
   publique com `stapler` pendente.
5. Reexecutar uma tag que já publicou: o workflow edita a release existente e sobe o DMG com
   `--clobber` em vez de falhar.

## Rodar na máquina do Helder

Mesmo script, mesmas credenciais por ambiente. No Mac, o atalho existente continua valendo: em vez
dos três secrets da API key, exporte só `NOTARY_KEYCHAIN_PROFILE=notary` (perfil criado uma vez com
`xcrun notarytool store-credentials notary --apple-id <id> --team-id K6M85697D4`; a senha fica no
keychain, não no repo).

```sh
git fetch --tags origin && git checkout v0.3.0-alpha.12      # o build é do commit da tag
export APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
export NOTARY_KEYCHAIN_PROFILE=notary                        # .p8 e .p12 ficam dispensados
sh scripts/release-macos.sh --check --tag v0.3.0-alpha.12
sh scripts/release-macos.sh --tag v0.3.0-alpha.12
```

Com `NOTARY_KEYCHAIN_PROFILE` o script não pede `APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD`:
o certificado já está no keychain e o `codesign` o encontra pela identidade. (No CI é o contrário: os
`.p12` vêm dos secrets e são importados por um passo do workflow.)

Sem credenciais nenhuma, o ensaio mecânico é `sh scripts/release-macos.sh --dry-run --tag
v0.3.0-alpha.12`: builda **ad-hoc** (a identidade é limpa mesmo que esteja no ambiente, para o ensaio
nunca assinar de verdade), monta o DMG, confere o layout e imprime os comandos de assinatura que
seriam executados. Nada é assinado, notarizado ou publicado.

## O que o script faz e por quê

- **`APPLE_SIGNING_IDENTITY` tem precedência** sobre `bundle.macOS.signingIdentity` do
  `tauri.conf.json`, que fica em `"-"` para o build local continuar funcionando em qualquer Mac. A
  identidade não é segredo; ainda assim ela não mora no repositório.
- **Hardened runtime** ligado (`bundle.macOS.hardenedRuntime`): é o padrão do Tauri 2 e explícito no
  config.
- **Sem arquivo de entitlements no app.** O app não usa JIT próprio (o WKWebView roda o JavaScript
  no `com.apple.WebKit.WebContent`, assinado pela Apple), não carrega dylib de terceiro e só dá
  `spawn` em `dotnet`, `csharp-ls` e `netcoredbg`. Se um dia precisar, o caminho é
  `bundle.macOS.entitlements` com o mínimo, nunca um plist inteiro copiado.
- **O runtime privado embutido tem entitlements próprios.** `scripts/vendor-analysis.sh` assina o
  muxer `Contents/Resources/dotnet/dotnet` com `src-tauri/resources/dotnet.entitlements`
  (`allow-jit` + `disable-library-validation`) e os 13 dylibs sem entitlements, todos com hardened
  runtime e timestamp. Sem `disable-library-validation` o `libhostfxr` não carrega sob hardened
  runtime; sem `allow-jit` o CLR não JITa. O netcoredbg embutido segue o mesmo desenho.
- **Auditoria de todos os Mach-O.** O script lista cada Mach-O do `.app` pelos quatro primeiros
  bytes — thin 32/64 bits nas duas endiannesses e os contêineres fat/fat64, inclusive os de campos
  trocados; `cafebabe` é desambiguado do `.class` do Java pelo `nfat_arch` (pequeno num binário
  universal, 45+ num class file), porque `file` responde `data` para fat64. Arquivo com menos de 8
  bytes é ignorado, caminho com quebra de linha e symlink que sai do `.app` fazem a auditoria falhar —
  hoje 18: binário
  principal, 15 do runtime .NET e 2 do netcoredbg — e exige, por arquivo, inclusive a verificação
  criptográfica `codesign --verify --strict`,
  `flags=0x10000(runtime)`, `Timestamp=` e `Authority=Developer ID Application: <nossa identidade>`.
  A única exceção é `createdump`, que o runtime oficial traz assinado pela Microsoft
  (`Developer ID Application: Microsoft Corporation (UBF8T346G9)`); qualquer outra autoridade
  estranha aborta o release. Dylibs não têm `TeamIdentifier`, por isso a comparação é por
  `Authority`.
- **A referência de integridade do par embutido é a assinatura do app.** csharp-ls e o runtime
  privado viajam em `Contents/Resources` e são cobertos pela assinatura Developer ID + notarização
  do `.app`/DMG. O host **não** compara contra hash fixado em runtime; o fingerprint
  (tamanho/mtime) só detecta mudança durante a sessão, e o SHA-256 exibido no SetupGate/Workspace
  tools é informativo.
- **O `.p8` vive no processo principal.** O caminho é definido antes de o arquivo existir
  (`$RUNNER_TEMP/notary-key.<pid>/AuthKey_<id>.p8`, diretório 0700 criado com `umask 077`) e o arquivo
  é criado no shell do script, não em `$(...)`: `notarize` cria a chave antes de capturar a saída do
  `notarytool`, então o `trap EXIT` sempre apaga arquivo e diretório, inclusive quando a notarização
  falha ou é rejeitada.
- **Notarização explícita, não a do Tauri.** Nada de `APPLE_ID`/`APPLE_PASSWORD` no ambiente (o
  Tauri notarizaria por conta própria e o build passaria a depender de segredo próprio). A ordem é:
  `.app` zipado com `ditto` → `notarytool submit --wait` → `stapler staple`/`validate` no `.app` →
  DMG montado com `hdiutil` → `codesign` no DMG → notarizar/grampear/validar o DMG.
- **`hdiutil` em vez de `--bundles dmg`.** Só `--bundles app` no build: o bundler de DMG refaz o
  `.app` a partir do binário e descartaria o ticket grampeado. O layout do DMG (volume `OpenRidr`,
  o app e o link para `/Applications`) é o mesmo do release anterior.
- **Temporários em `$RUNNER_TEMP`.** Montar o DMG e copiar o arquivo final fora de pasta sincronizada
  (iCloud/file provider pode travar `codesign`/`hdiutil` no arquivo enquanto ele sincroniza).

## O que o script verifica (todas têm de passar)

App e DMG: `codesign --verify --deep --strict`, `Authority`/`TeamIdentifier`/`flags` do app,
`spctl --assess` no app e no DMG, `stapler validate` nos dois, o app **dentro** do DMG montado
(`codesign --verify --deep --strict` e `spctl`), `file` = Mach-O arm64, versão do `defaults read`
igual à tag e `shasum -a 256`. A montagem é desfeita mesmo em erro.

Se a notarização for rejeitada, o log vem de `xcrun notarytool log <id>` (o script faz isso sozinho)
e aponta o item exato (`signature`, `hardened runtime`, `entitlements`, binário aninhado sem
assinatura). Corrija, refaça o build e notarize de novo — nunca publique com `stapler` pendente.

## Abrir como quem baixou (quarentena)

A prova que importa é o caminho do usuário: `xattr -w com.apple.quarantine` no DMG, montar, copiar o
`.app` e abrir por `open`. No macOS 15+ isso (a) faz o app rodar **translocado**
(`…/T/AppTranslocation/…/d/OpenRidr.app`, normal e esperado) e (b) apresenta **uma** confirmação de
primeira abertura — "é um app baixado da internet… A Apple buscou por software malicioso e não
detectou nenhum" + botão *Abrir*. Esse diálogo é do sistema, não uma recusa do Gatekeeper: confira
`spctl --assess` = *accepted, source=Notarized Developer ID* e o log do `CoreServicesUIAgent`
(`present code-evaluation prompt`, `XProtect suppress first launch warning: true`).

Dois enganos comuns ao automatizar isso: o diálogo **não** aparece na árvore de acessibilidade de
nenhum app (`CoreServicesUIAgent` expõe zero superfícies em `list-surfaces`), então é clique físico
(`agent-desktop mouse-click --xy x,y --headed`); e enquanto ele está aberto o processo fica parado em
`_dyld_start` com ~96 KB de footprint (`sample`), o que parece travamento do app mas é o kernel
esperando a decisão. Sem o xattr de quarentena o app abre direto — compare os dois para não atribuir
ao produto um comportamento do macOS.
