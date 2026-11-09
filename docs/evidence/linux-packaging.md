# Evidência — pacote Linux: .deb, AppImage e instalação user-local (issue #322)

Data: 2026-11-09. Branch `feat/linux-packaging` (base `origin/main` em `234ff02`). Spec: corpo da
issue [#322](https://github.com/Heldinhow/openridr/issues/322), tickets #323, #324 e #325.
Decisão: [`../decisions.md`](../decisions.md) (2026-11-09).

Ambiente: **Arch Linux ARM**, `aarch64`, Wayland/Hyprland. `bun` 1.3.9, `cargo` 1.99.0. Nenhum
`sudo`, nenhum `dpkg` na máquina, nenhuma dependência nova.

## Build (#323)

`bun run tauri build --bundles deb` a partir desta worktree (target frio, 1m13s de compilação):

- `src-tauri/target/release/bundle/deb/OpenRidr_0.3.0-alpha.11_arm64.deb`, **12.065.400 bytes**
  (11,51 MiB). Conteúdo (`bsdtar -xOf <deb> data.tar.gz | bsdtar -tzf -`):

  ```text
  usr/bin/openridr
  usr/lib/OpenRidr/netcoredbg/{netcoredbg,libdbgshim.so,ManagedPart.dll,...}
  usr/lib/OpenRidr/THIRD-PARTY-NOTICES.md
  usr/share/applications/OpenRidr.desktop
  usr/share/icons/hicolor/32x32/apps/openridr.png
  usr/share/icons/hicolor/128x128/apps/openridr.png
  usr/share/icons/hicolor/256x256/apps/openridr.png
  ```

- `control` lido do pacote:

  ```text
  Package: open-ridr
  Version: 0.3.0-alpha.11
  Architecture: arm64
  Installed-Size: 32764
  Maintainer: openridr
  Section: devel
  Priority: optional
  Depends: libwebkit2gtk-4.1-0, libgtk-3-0 | libgtk-3-0t64, libwebkit2gtk-4.1-0, libgtk-3-0
  Description: OpenRidr desktop shell
  ```

  O bundler **acrescenta** `libwebkit2gtk-4.1-0` e `libgtk-3-0` à lista declarada, então os dois
  aparecem duas vezes; `dpkg` trata `Depends` como conjunto e a alternativa `| libgtk-3-0t64` (a
  renomeação do Ubuntu 24.04+) continua valendo. Não há como suprimir a duplicata sem fork do
  bundler.

- O `.desktop` do pacote é o template do repositório, sem substituição:

  ```text
  [Desktop Entry]
  Type=Application
  Name=OpenRidr
  GenericName=C# IDE
  Comment=IDE for C# solutions and .NET projects
  Exec=openridr %F
  Icon=openridr
  Terminal=false
  Categories=Development;IDE;
  Keywords=dotnet;csharp;ide;editor;debugger;
  StartupWMClass=openridr
  ```

- **Ícone de 256 px.** `128x128@2x.png` virava `hicolor/256x256@2/apps/openridr.png` — um diretório
  de tamanho que a especificação de tema não conhece. O `tauri.linux.conf.json` declara a lista de
  ícones do Linux com `icons/256x256.png` (cópia do mesmo PNG de 256 px) e o pacote passa a ter
  `hicolor/256x256/apps/openridr.png`. O `tauri.conf.json` base e o macOS não mudam.
- **`StartupWMClass`** conferido no app rodando: `hyprctl clients` reporta `class: openridr`, não o
  nome do produto.

`bun run tauri build --bundles appimage` (reusa o binário de release, 10,9 s de compilação):

- `src-tauri/target/release/bundle/appimage/OpenRidr_0.3.0-alpha.11_aarch64.AppImage`,
  **117.643.784 bytes** (112,19 MiB).
- Downloads do build, todos da lista do próprio bundler: `AppRun-aarch64`,
  `linuxdeploy-aarch64.AppImage`, `linuxdeploy-plugin-appimage-aarch64.AppImage`. Nenhuma
  ferramenta instalada no sistema, nenhuma dependência nova no repositório.
- `OpenRidr.AppDir/` tem `OpenRidr.desktop` (o mesmo template), `.DirIcon`, `openridr.png`,
  `usr/share/icons/hicolor/{32x32,128x128,256x256}/apps/openridr.png` e
  `usr/lib/OpenRidr/netcoredbg/`.
- O AppImage **abre**: `OpenRidr_0.3.0-alpha.11_aarch64.AppImage --appimage-extract-and-run` com
  `OPENRIDR_HOME=/tmp/openridr-linux-packaging/home-appimage` criou uma janela
  (`hyprctl clients`: `class: openridr`, `title: OpenRidr`). Sem `fuse2` nesta máquina, então o
  caminho "duplo clique no gerenciador de arquivos" não foi exercitado.

## Host (#324)

- `cargo test -p openridr-host --lib -- bundled`: **11 passaram, 3 ignorados** (os leaves isolados,
  que rodam em processo próprio). Os novos:
  - `the_package_layout_is_the_directory_the_shell_resolved` — `resource_dir` = `…/usr/lib/OpenRidr`
    com o binário em `…/usr/bin/openridr` (nomes diferentes, que é o ponto).
  - `the_user_local_layout_sits_beside_the_binary` — sem `resource_dir`, e com um `resource_dir`
    que não existe, o root é `<exe dir>/../lib/<nome do binário>`.
  - `a_build_tree_has_no_resource_root` — nenhum dos dois candidatos: `None`, o setup bloqueia.
  - `the_registered_package_directory_resolves_the_bundled_adapter` — com o diretório do pacote
    registrado, `resources_root()` é `…/usr/lib/OpenRidr` e `debugger::bundled_binary()` aponta para
    `…/usr/lib/OpenRidr/netcoredbg/netcoredbg`. Que esse adaptador reporte `ready` com
    `bundled: true` é o que `debugger::linux_bundled_adapter_is_ready_and_a_missing_or_damaged_one_blocks`
    (#318) prova uma vez resolvido o caminho.
- `cargo clippy -p openridr-host --lib -- -D warnings`: limpo. `cargo fmt --all -- --check`: limpo.
- `cargo clippy -p openridr-host --all-targets -- -D warnings` **falha nesta máquina** por 7 avisos
  de código morto em `crates/host/tests/host.rs` (`wait_publication`, `wait_for_dap`,
  `write_netcoredbg`, …), helpers usados só por testes `#[cfg(target_os = "macos")]`. O arquivo não
  é tocado por esta frente e o CI roda o clippy no `macos-15`, onde eles são usados.

## Instalação user-local (#325)

- `tests/shell/install-app-linux.test.sh` sob **dash 0.5.12** (`/tmp/dash-install/bin/dash`, porque o
  `/bin/sh` do Arch é bash): **24 checks verdes** — instalação completa (binário, payload, `.desktop`,
  três tamanhos de ícone), `Exec` absoluto com `%F`, `Categories=Development;IDE;`, ausência de
  `MimeType`, `~/.local` real intocado com `--prefix`, reinstalação que troca o payload em vez de
  mesclar, prefixo relativo recusado, binário ausente recusado com mensagem, `--prefix` sem valor e
  argumento desconhecido recusados.
- Instalação real num prefixo temporário (`/tmp/openridr-linux-packaging/prefix`) com o binário de
  release e o payload embutido: `install-app-linux: installed openridr at …/prefix/bin/openridr`.

## App real (o que abriu)

Janela levada ao workspace 7, `flock` em `/tmp/openridr-linux-packaging/.gui.lock`,
`OPENRIDR_HOME=/tmp/openridr-linux-packaging/home`:

- **Layout do pacote:** o deb extraído em `/tmp/openridr-linux-packaging/deb-root` e
  `usr/bin/openridr` executado dali (o Tauri resolve `resource_dir()` relativo ao executável, então
  o layout é o mesmo de uma instalação em `/usr`) → janela criada, um cliente.
- **Layout user-local:** `…/prefix/bin/openridr fixtures/OrderDesk/OrderDesk.sln` → janela criada.
  Aqui o `resource_dir()` é `<prefixo>/lib/OpenRidr` (nome do produto, com maiúscula), que não
  existe; quem responde é o irmão do binário, `<prefixo>/lib/openridr`.
- **Instância única:** uma segunda execução (`…/prefix/bin/openridr fixtures/OrderDesk`) saiu com
  código 0 e continuou **um** cliente — a janela existente foi trazida para frente, sem segundo
  processo.

## Não verificado

- **O `.deb` instalado num Debian/Ubuntu real.** Não há `dpkg` nesta máquina (Arch); o pacote foi
  inspecionado por listagem (`bsdtar`), não instalado. O `Depends` declarado não foi exercitado por
  um resolvedor de verdade.
- **`x86_64`.** Nenhum build nem execução; o `vendor-netcoredbg.sh` tem o asset `linux-amd64`
  fixado, mas nada foi construído nessa arquitetura.
- **`fuse2`.** O AppImage foi aberto com `--appimage-extract-and-run`; o caminho normal (montagem
  por FUSE) não foi exercitado.
- **O texto da tela de setup.** O barramento AT-SPI está recusando conexão nesta sessão
  (`Couldn't connect to accessibility bus`), então o `agent-desktop` não consegue ler a árvore de
  acessibilidade; as capturas foram tiradas por `grim`, mas esta sessão não tem modelo que leia
  imagem. O que está provado é o encadeamento: o pacote põe o adaptador em
  `usr/lib/OpenRidr/netcoredbg/netcoredbg`, o host resolve esse diretório (teste acima) e o app
  instalado pelos três caminhos abre a janela. Ler `setup.state = ready` e `debug.adapterState =
  ready` na tela fica para a verificação com `agent-desktop` num ambiente com AT-SPI.
- **Debug parando em breakpoint** com o adaptador embutido: depende da sessão acima (abrir a
  solution, confiar, iniciar o debug) e não foi executado nesta sessão.
- **Ícone no menu de aplicativos.** A entrada foi escrita no prefixo temporário, que não está no
  `XDG_DATA_DIRS` da sessão; para ver o ícone no menu seria preciso instalar em `~/.local` de
  verdade, o que esta frente não faz.
- **macOS.** Nenhum build nesta máquina; a configuração base (`tauri.conf.json`) e o caminho do
  macOS não foram tocados por esta frente, e o CI cobre o build de lá.
