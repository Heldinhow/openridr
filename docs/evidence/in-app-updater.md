# Evidência — atualização dentro do app (issue #55)

Data: 2026-11-07. Alvo: macOS 26.1 arm64, app de release local (ad-hoc) instalado em
`~/Applications-test/OpenRidr-issue55`. Tudo abaixo foi executado; o que não rodou está dito como
não verificado.

## O que a prova mostra

Um app instalado detecta uma versão nova, mostra versão e notas, baixa com progresso, instala,
reinicia depois da decisão sobre buffer não salvo e volta rodando a versão nova — e recusa um
manifesto com assinatura adulterada sem tocar no bundle.

## Como a prova foi montada

Nada de release publicado e nada de tag: dois builds locais e um servidor em loopback.

1. **Build "instalado" (alpha.11).** `bunx tauri build --target aarch64-apple-darwin --bundles app`
   com `--config` sobrescrevendo **só** o endpoint do updater para
   `http://127.0.0.1:4817/latest.json` e `dangerousInsecureTransportProtocol: true`. O override não é
   commitado: o `tauri.conf.json` do repositório continua com o endpoint https do canal. O bundle
   (166 MiB, `codesign --verify --deep --strict` ok) foi copiado para
   `~/Applications-test/OpenRidr-issue55/OpenRidr.app`.
2. **Build "novo" (alpha.12).** Mesmo comando, com a versão do `tauri.conf.json` em
   `0.3.0-alpha.12` localmente e revertida em seguida (`git checkout`). Empacotado com
   `tar --no-mac-metadata -czf OpenRidr.app.tar.gz OpenRidr.app` (raiz `OpenRidr.app/`) e assinado
   com a chave real: `tauri signer sign --app-version 0.3.0-alpha.12`, cujo trusted comment ficou
   `timestamp:...	file:OpenRidr.app.tar.gz	version:0.3.0-alpha.12`.
3. **Manifesto.** `latest.json` com os mesmos campos que `scripts/release-updater-manifest.sh`
   emite (version, notes, pub_date, `platforms.darwin-aarch64.{signature,url}`). O script exige URL
   https e por isso não foi usado aqui; a assinatura saiu do `.sig` real, não de um valor plantado.
4. **Servidor.** `python3` em `127.0.0.1:4817`, só loopback, entregando o `.tar.gz` em blocos de
   256 KiB para o progresso ser observável.
5. **App.** `desktop.sh up` da skill `verify-openridr` com `OPENRIDR_HOME` isolado
   (um `<tmp>/verify-openridr-issue55-desktop/home`, nunca o app home do usuário) e a fixture OrderDesk em argv; uma instância por vez,
   `pgrep -x openridr` vazio antes de abrir. Dirigido por `agent-desktop`.

## Positiva

| Passo | Resultado |
|---|---|
| Checagem automática no startup | o modal abriu sozinho ~3 s depois do primeiro paint: "OpenRidr **0.3.0-alpha.12** is available; you have 0.3.0-alpha.11." com as notas do `RELEASE-ALPHA.md` ([dialog.png](in-app-updater/dialog.png)) |
| "Later" | fecha o modal e o app continua no workspace, buffer alterado intacto |
| Menu **Check for Updates…** | reabre o modal e checa na hora (o item fica logo abaixo de "About OpenRidr") |
| "Download and install" | barra de progresso com percentual ("Downloading… 30%") ([progress.png](in-app-updater/progress.png)) |
| Fim do download | "OpenRidr 0.3.0-alpha.12 is installed. Restart to start using it; unsaved files are handled before the app closes." com **Later** / **Restart now** ([installed.png](in-app-updater/installed.png)) |
| "Restart now" com buffer sujo | abre o prompt do ⌘Q: "Close OpenRidr with unsaved changes? 1 modified buffer has unsaved changes." com **Save all** / **Keep editing** / **Discard and close app** ([restart-guard.png](in-app-updater/restart-guard.png)) |
| "Keep editing" | o app continua vivo (mesmo PID), nada reinicia |
| "Restart now" → "Discard and close app" | o app sai e volta com **PID novo** (18541 → 15073), com o mesmo `OPENRIDR_HOME` e a mesma solução em argv |
| Versão em execução depois do restart | menu **Check for Updates…** → "OpenRidr 0.3.0-alpha.12 is the newest version." ([restarted-version.png](in-app-updater/restarted-version.png)); `CFBundleShortVersionString` do bundle em disco também `0.3.0-alpha.12` |

O `current` do modal vem de `app.package_info().version`, que é a versão compilada no binário — é
por isso que essa linha prova qual versão está **rodando**, não só qual está no disco.

## Negativa (assinatura adulterada)

Manifesto trocado para anunciar `0.3.0-alpha.13` com um caractere da assinatura base64 alterado,
servindo o mesmo `.tar.gz`:

- o modal ofereceu a versão nova ("OpenRidr 0.3.0-alpha.13 is available; you have 0.3.0-alpha.12.");
- "Download and install" baixou e **recusou**: "Could not install the update: Invalid encoding in
  minisign data" ([refused.png](in-app-updater/refused.png));
- o bundle em disco continuou `0.3.0-alpha.12` e o PID não mudou: nada foi trocado.

O download acontece antes da verificação (é o desenho do plugin), então a recusa custa a
transferência inteira. O manifesto bom foi restaurado depois.

## Bug encontrado por esta prova (e corrigido)

A primeira tentativa de instalar falhou com
`Could not install the update: failed to unpack '._OpenRidr.app' into '.../tauri_updated_app...'`.

Causa medida: o `tar` do macOS grava uma entrada **AppleDouble** `._OpenRidr.app` para os xattrs do
bundle (`com.apple.provenance`), e o leitor tar do plugin **não** a funde de volta em xattr — ele a
entrega como entrada, cujo primeiro componente é o arquivo inteiro, e o plugin (que descarta o
primeiro componente de cada caminho) tenta desempacotá-la no diretório temporário. O `bsdtar` esconde
essas entradas ao listar e ao extrair, então `tar -tzf` mostrava o arquivo limpo (455 entradas, zero
`._`); quem as revela é um leitor que não as interpreta (`pax -r`, ou o crate `tar`).

Correção em `scripts/release-macos.sh`: empacotar com `tar --no-mac-metadata` e, na verificação,
ler o **primeiro header do fluxo cru** (`gzip -dc | dd bs=512 count=1 | dd bs=100 count=1`) e recusar
a release quando ele começa com `._`. Guarda em `scripts/release-selfcheck.sh` com um xattr plantado
no bundle do fixture: sem a flag o self-check reprova com
`starts with the AppleDouble entry '._OpenRidr.app', which the plugin cannot unpack`; com a flag
passa. A prova acima (positiva e negativa) foi refeita com o arquivo corrigido.

## Comandos e saídas (resumo)

```text
$ TAURI_CONFIG=... bunx tauri build --target aarch64-apple-darwin --bundles app --config '{"plugins":{"updater":{"endpoints":["http://127.0.0.1:4817/latest.json"],"dangerousInsecureTransportProtocol":true}}}'
Finished 1 bundle at: .../bundle/macos/OpenRidr.app (166.34 MiB)

$ plutil -extract CFBundleShortVersionString raw -o - .../OpenRidr.app/Contents/Info.plist
0.3.0-alpha.11            # instalado; depois do update: 0.3.0-alpha.12

$ strings -a .../Contents/MacOS/openridr | grep -c 127.0.0.1:4817
1                          # o endpoint do override é o que o binário usa

$ openssl base64 -d -A < OpenRidr.app.tar.gz.sig | sed -n 3p
trusted comment: timestamp:1794062646	file:OpenRidr.app.tar.gz	version:0.3.0-alpha.12

$ python3 -c "import tarfile; print(len(tarfile.open('OpenRidr.app.tar.gz').getnames()))"
455                        # sem entrada AppleDouble depois da correção

$ sh scripts/release-selfcheck.sh            # verde (com a guarda da AppleDouble)
$ sh scripts/release-updater-manifest-selfcheck.sh   # verde
$ sh scripts/updater-pubkey-selfcheck.sh     # verde
$ cargo test --manifest-path src-tauri/Cargo.toml    # 6 passed
$ cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings   # limpo
$ bun run test:unit                          # 95 passed
```

## Doctor e limpeza

- `sh .agents/skills/verify-openridr/scripts/doctor.sh <bundle>` rodado **depois** da prova, não
  antes (falha de processo desta sessão): bun, cargo, `node_modules`, SDK 8.0.408, Chrome,
  Acessibilidade e Gravação de Tela todos `ok`. O único `note` — bundle `0.3.0-alpha.12` vs
  `package.json` `0.3.0-alpha.11` — é o resultado esperado da prova: o bundle sob teste é o que o
  update instalou, e o `package.json` é o do checkout.
- Fim da sessão: app fechado pelo item **Quit OpenRidr** do menu (não `pkill`), `desktop.sh down`
  diz `stopped`, `pgrep -lx openridr` vazio, nenhuma porta de verificação ouvindo e
  `launchctl getenv OPENRIDR_HOME` vazio. O servidor local da prova foi parado pelo próprio PID.

## Não verificado

- **Canal real do GitHub** (`updater-alpha`): a prova usou um servidor local. O upload dos assets e
  o `latest.json` publicado só acontecem no próximo run de release (tag `v*`), que não foi disparado
  nesta sessão. O `release.yml` foi lido e o passo novo foi validado por `actionlint`, mas o caminho
  `gh release upload` → download pelo app **não rodou**.
- **Assinatura Developer ID + notarização + grampo no artefato de update**: a prova usou builds
  ad-hoc; `verify_updater_artifact` (codesign/spctl/stapler no app extraído) rodou apenas com os
  fakes do `release-selfcheck.sh`. Só um release real prova o caminho completo.
- **Gatekeeper/quarentena no app atualizado**: o `.tar.gz` local não tem `com.apple.quarantine`.
  O comportamento de primeira abertura de um update baixado da internet não foi medido.
- **TCC (Acessibilidade/Gravação de Tela) depois do update**: builds ad-hoc perdem a permissão a
  cada troca de bundle, então a prova não podia medir isso; com Developer ID a expectativa é que a
  permissão persista (mesmo bundle ID e mesma identidade), mas isso é expectativa, não medição.
- **Proxy corporativo no updater**: o reqwest do plugin usa `rustls-platform-verifier` e
  `system-proxy` (lido no código-fonte da crate), mas nenhuma requisição passou por proxy nesta
  sessão.
- **Windows e Linux**: fora do alvo verificado do projeto.
- **Leitor de tela real**: a árvore do `agent-desktop` expõe o modal (dialog, `heading`, botões com
  `ref_id`), mas ordem de tabulação e anúncio de leitor de tela não foram medidos.
