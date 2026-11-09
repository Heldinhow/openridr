# Instalar o OpenRidr como app local (Linux e macOS ARM64)

## Linux (deb, AppImage ou `~/.local`)

O build do Linux produz dois pacotes e o repositório traz um terceiro caminho, sem root:

| Distro | Artefato | Como instalar |
|---|---|---|
| Debian, Ubuntu, Mint | `OpenRidr_<versão>_arm64.deb` | `sudo apt install ./OpenRidr_<versão>_arm64.deb` |
| Qualquer distro, sem root | `OpenRidr_<versão>_aarch64.AppImage` | `chmod +x` e execute; sem `fuse2`, `--appimage-extract-and-run` |
| Arch e derivados | `bun run app:install:linux` | instala em `~/.local` |

```sh
bun run tauri build          # src-tauri/target/release/bundle/{deb,appimage}/
bun run app:install:linux    # ~/.local, sem root
```

- **O `.deb`** declara `Depends: libwebkit2gtk-4.1-0, libgtk-3-0 | libgtk-3-0t64` (a alternativa
  cobre a renomeação `t64` do Ubuntu 24.04+) e instala o binário em `/usr/bin/openridr`, o payload
  em `/usr/lib/OpenRidr/`, a entrada em `/usr/share/applications/OpenRidr.desktop` e o ícone em
  `/usr/share/icons/hicolor/{32x32,128x128,256x256}/apps/openridr.png`. Remover:
  `sudo apt remove open-ridr`.
- **O `.AppImage`** roda sem instalar nada. Ele precisa de `fuse2`; sem ele, use
  `./OpenRidr_<versão>_aarch64.AppImage --appimage-extract-and-run`.
- **`bun run app:install:linux`** compila com `tauri build --no-bundle` e copia para
  `~/.local/bin/openridr`, `~/.local/lib/openridr/netcoredbg/`,
  `~/.local/share/applications/openridr.desktop` (com o `Exec` absoluto, porque sessão gráfica
  raramente tem `~/.local/bin` no `PATH`) e `~/.local/share/icons/hicolor/…`. `--prefix <dir>`
  instala em outro lugar e nada é escrito fora dele. Remover:

  ```sh
  rm -rf ~/.local/bin/openridr ~/.local/lib/openridr \
    ~/.local/share/applications/openridr.desktop ~/.local/share/icons/hicolor/*/apps/openridr.png
  ```

- **Onde o app procura o netcoredbg.** O shell passa o `resource_dir()` ao host; o host usa esse
  diretório quando existe (`/usr/lib/OpenRidr` no deb, `$APPDIR/usr/lib/OpenRidr` no AppImage) e cai
  para o irmão do binário (`<prefixo>/lib/openridr`) quando não existe. Nenhum dos três caminhos
  baixa o adaptador; se ele faltar, o setup diz para reinstalar o app.
- **Abrir uma solução:** `openridr OrderDesk.sln`, ou arrastar o arquivo para o ícone — a entrada
  declara `Exec=… %F`. Com o app já aberto, a segunda execução só traz a janela para frente e abre
  o arquivo pedido.
- **Fora do Linux hoje:** o updater ("Check for updates" falha; não há canal para Linux), o
  `csharp-ls` baixado no primeiro uso e a associação de `.sln`/`.slnx`/`.csproj` no gerenciador de
  arquivos (o `.desktop` não declara `MimeType`).
- **Não verificado:** `.deb` instalado num Debian/Ubuntu real (o host verificado é Arch Linux ARM,
  sem `dpkg`), `x86_64` e o AppImage por FUSE. Detalhes em
  [evidence/linux-packaging.md](evidence/linux-packaging.md).

## macOS ARM64

Uso pessoal, sem assinatura nem notarização. O DMG de release é assinado com Developer ID e
notarizado; este script continua ad-hoc — ver [release-signing.md](release-signing.md).

```sh
bun run app:install
```

O comando faz `tauri build --bundles app` e copia o bundle para `/Applications/OpenRidr.app`, substituindo a cópia anterior. Para atualizar, rode de novo depois de subir a versão em `package.json` e `src-tauri/tauri.conf.json`.

- **Assinatura:** ad-hoc (`signingIdentity: "-"`), necessária para o Apple Silicon executar o binário. Não é Developer ID.
- **Gatekeeper:** o app gerado por este script não é notarizado. Na primeira abertura, use botão direito → Abrir, ou remova a quarentena com `xattr -dr com.apple.quarantine /Applications/OpenRidr.app`.
- **Ícone:** provisório, o mesmo `src-tauri/icons/icon.png` do repositório, convertido para `.icns`/`.ico`.
- **Fora deste fluxo:** instalador público e Windows/macOS Intel. Assinatura Developer ID e notarização existem apenas no DMG de release (ver [release-signing.md](release-signing.md)); esse DMG roda só em Apple Silicon e, por ser baixado, **mostra o diálogo de primeira abertura** do macOS (na primeira vez).

## Primeira abertura de um app baixado (DMG de release)

App baixado por Safari/Chrome carrega a marca de quarentena do macOS, e o sistema **não inicia**
o OpenRidr até alguém responder ao diálogo "… é um app baixado da internet. Tem certeza de que
deseja abrí-lo?". Medido no alpha.11 desta máquina (ver
[evidence/first-open.md](evidence/first-open.md)):

- com o app já arrastado para Aplicações: o diálogo aparece em **~0,5 s** e a janela em **~1,4 s
depois do clique**;
- aberto **direto da janela do DMG**: **~5–7 s** até a janela depois do clique, porque o sistema
  lê os 164 MB do app de dentro da imagem comprimida (`ditto` do mesmo app: 1,95 s).

Então: arraste para **Applications** antes de abrir, e na primeira abertura clique em **Abrir** (ou
tecle Enter). Não é lentidão do app nem verificação de assinatura que dê para otimizar — é o
macOS esperando a decisão. Na segunda abertura não há diálogo nem esse custo.

Para medir de novo numa máquina: `sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app`
reproduz a condição "app no disco" (cópia em diretório temporário, quarentena nova a cada rodada).
O caminho "abrir de dentro do DMG" não é coberto pelo script, e ele recusa um build ad-hoc (o de
`bun run app:install`); `--allow-unnotarized` mede mesmo assim, com a ressalva de que o diálogo
será outro.

## Abrir do Finder e do terminal

Nada disto é instalado automaticamente: nem o app nem o `install-app.sh` mexem no PATH, no shell
ou em links. O app instalado por este script também não é notarizado, então vale a mesma nota de
Gatekeeper acima.

- **Abrir Com:** o bundle declara `.sln`, `.slnx`, `.csproj` (Editor, padrão) e pastas (Alternate,
  não vira padrão para pastas). Depois de instalar, use Finder → Obter Informações → Abrir com →
  OpenRidr → Alterar Todos para tornar o app padrão de um tipo.
- **Arrastar pasta para o Dock:** arraste a pasta para o ícone do OpenRidr no Dock. O app abre a
  solution ou o projeto que ela contém; com duas solutions, a escolha é explícita.
- **Rebuild obrigatório:** as associações vêm do `Info.plist` gerado. Depois de mudar
  `tauri.conf.json`, gere de novo com `bun run app:install` (ou
  `bun run tauri build --debug --bundles app` para o bundle de debug em
  `src-tauri/target/debug/bundle/macos/OpenRidr.app`).
- **Cache do LaunchServices:** se o Finder ainda não mostrar o app, registre o bundle de novo:
  `/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/OpenRidr.app`
  e, se preciso, reabra o Finder.
- **Terminal:** `scripts/openridr <caminho>` valida que o caminho existe e que é pasta ou
  `.sln`/`.slnx`/`.csproj`, imprime qual bundle vai abrir e chama `open -a OpenRidr <caminho>`,
  que reusa a instância já aberta. Uso inválido, extensão não suportada ou caminho ausente saem
  com erro em `stderr` e código diferente de 0, sem chamar `open`.
- **Link manual no PATH (opcional, sua decisão):**

  ```sh
  mkdir -p ~/.local/bin && ln -s "$PWD/scripts/openridr" ~/.local/bin/openridr
  ```

  Ou `/usr/local/bin`, se ele já estiver no seu PATH. O link aponta para este repositório: se o
  repositório mudar de lugar, refaça o link.
- **Dev, bundle de debug:** `OPENRIDR_APP` aponta o launcher para outro bundle:

  ```sh
  OPENRIDR_APP="$PWD/src-tauri/target/debug/bundle/macos/OpenRidr.app" scripts/openridr fixtures/OrderDesk/OrderDesk.sln
  ```

  O bundle de debug usa o mesmo `CFBundleIdentifier` do app instalado. Se um deles já estiver
  aberto, o LaunchServices reusa essa instância, não a outra. Feche a instância antes de testar.
  `open` também não repassa variáveis do shell: para isolar `OPENRIDR_HOME`, use
  `open --env OPENRIDR_HOME=<dir> -a <bundle> <arquivo>` direto, sem o launcher.

- **Teste isolado do LaunchServices (sem tocar no app instalado):** copie o bundle, troque o
  identificador, assine ad-hoc e abra com `--env`. O LaunchServices trata a cópia como outro app,
  então a instância de `/Applications` não é reutilizada.

  ```sh
  verify=$(mktemp -d)/OpenRidrVerify.app
  ditto src-tauri/target/debug/bundle/macos/OpenRidr.app "$verify"
  plutil -replace CFBundleIdentifier -string com.openridr.verify.openwith "$verify/Contents/Info.plist"
  codesign --force --deep --sign - "$verify"
  open --env OPENRIDR_HOME="$PWD/.scratch/host-home" -a "$verify" fixtures/OrderDesk/OrderDesk.sln
  ```

  Repita o `open -a` com outro arquivo ou com a pasta `fixtures/OrderDesk` para provar a
  instância única. Encerre a cópia e apague o diretório temporário ao final; nada é gravado em
  `~/Library/Application Support/OpenRidr` com o `OPENRIDR_HOME` acima.
