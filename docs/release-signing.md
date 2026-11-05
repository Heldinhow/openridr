# Assinar e notarizar o DMG (macOS ARM64)

Vale só para o build de release. O build local (`bun run app:install`) continua ad-hoc e sem
notarização — ver [install-app.md](install-app.md).

Nada de segredo entra no repositório: a identidade é pública, a senha da Apple ID fica no keychain
do macOS (perfil `notary` do `notarytool`) e o certificado nunca é exportado.

## Pré-requisitos (uma vez por Mac)

```sh
security find-identity -v -p codesigning
# precisa listar: "Developer ID Application: Helder Santos (K6M85697D4)"

xcrun notarytool history --keychain-profile notary
# "No submission history." também é resposta válida: o perfil existe e autenticou.
```

O perfil é criado uma vez com `xcrun notarytool store-credentials notary --apple-id <id> --team-id K6M85697D4`
e grava Apple ID, Team ID e app-specific password no keychain. Não repita o comando no repo; não
exporte `APPLE_ID`, `APPLE_PASSWORD` nem `APPLE_CERTIFICATE`.

## Build assinado (sem notarizar pelo Tauri)

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
bun run tauri build --target aarch64-apple-darwin --bundles app
```

- `APPLE_SIGNING_IDENTITY` tem precedência sobre `bundle.macOS.signingIdentity` do
  `tauri.conf.json` (`tauri-cli/src/interface/rust.rs`), e o config fica em `"-"` para o build
  local continuar funcionando em qualquer Mac. A identidade não é segredo; ainda assim ela não
  precisa morar no repositório.
- **Hardened runtime** está ligado: é o padrão do Tauri 2 e está explícito em
  `bundle.macOS.hardenedRuntime`.
- **Sem arquivo de entitlements.** O app não usa JIT próprio (o WKWebView roda o JavaScript no
  `com.apple.WebKit.WebContent`, assinado pela Apple), não carrega dylib de terceiro e só dá
  `spawn` em `dotnet`, `csharp-ls` e `netcoredbg` como processos filhos. Sandbox do Mac App Store
  não se aplica. Se um dia precisar, o caminho é `bundle.macOS.entitlements` com o mínimo
  (ex.: `com.apple.security.cs.disable-library-validation`), nunca um plist inteiro copiado.
- Só `--bundles app` aqui: o bundler de DMG refaz o `.app` a partir do binário e descartaria o
  ticket grampeado. O DMG é montado no passo seguinte, depois do `stapler`.

## Notarizar e grampear o .app

```sh
app=src-tauri/target/aarch64-apple-darwin/release/bundle/macos/OpenRidr.app
ditto -c -k --keepParent --sequesterRsrc "$app" /tmp/OpenRidr-alpha5.zip
xcrun notarytool submit /tmp/OpenRidr-alpha5.zip --keychain-profile notary --wait
xcrun stapler staple "$app"
xcrun stapler validate "$app"
```

O `submit` imprime `status: Accepted` e um `id`. O zip é só o transporte: o ticket fica no `.app`.

## Montar, assinar e notarizar o DMG

```sh
stage=$(mktemp -d)
ditto "$app" "$stage/OpenRidr.app"
ln -s /Applications "$stage/Applications"
dmg=/tmp/OpenRidr_0.3.0-alpha.7_aarch64.dmg
hdiutil create -volname OpenRidr -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dmg"
codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$dmg"
xcrun notarytool submit "$dmg" --keychain-profile notary --wait
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"
```

Monte o DMG num diretório temporário e copie o arquivo final para `~/Downloads` só no fim: pasta
sincronizada (iCloud/file provider) pode travar `codesign`/`hdiutil` no arquivo enquanto ele
sincroniza.

Montar o DMG com `hdiutil` em vez de `--bundles dmg` é o que garante um DMG com o **app já
grampeado** dentro. O layout é o mesmo do release anterior (`CI=true` também pulava o AppleScript
do Finder): volume `OpenRidr`, o app e o link para `/Applications`.

## Verificação (todas têm de passar)

```sh
app=src-tauri/target/aarch64-apple-darwin/release/bundle/macos/OpenRidr.app
dmg=~/Downloads/OpenRidr_0.3.0-alpha.7_aarch64.dmg

codesign --verify --deep --strict --verbose=2 "$app"
codesign -dv --verbose=4 "$app" | grep -E "Authority|TeamIdentifier|flags"
spctl --assess --type execute -vv "$app"          # accepted, source=Notarized Developer ID
spctl --assess --type open --context context:primary-signature -vv "$dmg"
xcrun stapler validate "$app"
xcrun stapler validate "$dmg"

hdiutil attach "$dmg" -nobrowse -mountpoint /tmp/OpenRidrDmg
codesign --verify --deep --strict --verbose=2 /tmp/OpenRidrDmg/OpenRidr.app
spctl --assess --type execute -vv /tmp/OpenRidrDmg/OpenRidr.app
defaults read /tmp/OpenRidrDmg/OpenRidr.app/Contents/Info CFBundleShortVersionString
file /tmp/OpenRidrDmg/OpenRidr.app/Contents/MacOS/openridr   # Mach-O 64-bit arm64
shasum -a 256 "$dmg"
hdiutil detach /tmp/OpenRidrDmg
```


Se a notarização for rejeitada, o log vem de `xcrun notarytool log <id> --keychain-profile notary`
e aponta o item exato (`signature`, `hardened runtime`, `entitlements`, binário aninhado sem
assinatura). Corrija, refaça o build e notarize de novo — nunca publique com `stapler` pendente.
