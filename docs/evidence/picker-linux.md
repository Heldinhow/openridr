# Seletor nativo no Linux pelo portal XDG (#319)

Máquina: Arch Linux ARM, Wayland/Hyprland 0.57.1, `xdg-desktop-portal` 1.22.1 com o backend
`gtk` 1.15.3, `~/.dotnet` com 8.0.408 e 10.0.403. Branch `fix/issue-319-picker-zbus` sobre
`eff5d21` (o #320 já está na `main`). O app rodou **só** pelo binário desta worktree
(`target/debug/openridr`, via `bunx tauri dev`), com `OPENRIDR_HOME=/tmp/openridr-319` e o dev
server na porta 2420; a janela foi identificada pelo PID e levada ao workspace 8, e foco, input e
captura rodaram sob `flock /tmp/openridr-gui.lock`. Nada foi instalado em `$HOME` (a lista de
recentes do GTK foi alterada para o teste e restaurada do backup em seguida).

## O que foi verificado

### Testes automatizados

```sh
$ cargo test -p openridr-host --test host -- file_chooser
running 2 tests
test a_registered_file_chooser_owns_workspace_pick ... ok
test the_file_chooser_refuses_where_no_native_dialog_exists ... ok

$ cargo test -p openridr-host
483 passed; 3 failed; 13 ignored
# as 3 falhas são git::tests pré-existentes neste host (ver abaixo)

$ cargo test --manifest-path src-tauri/Cargo.toml
running 10 tests
test picker::tests::a_request_path_is_the_sender_without_the_colon_and_the_token ... ok
test picker::tests::tokens_are_unique_path_elements ... ok
test picker::tests::file_uris_become_local_paths_and_other_schemes_are_ignored ... ok
test picker::tests::the_filters_cover_solutions_projects_and_every_file ... ok
... 6 testes que já existiam ... ok
test result: ok. 10 passed; 0 failed

$ bun test tests/unit
156 pass, 0 fail

$ bun run build
tsc --noEmit + vite build ok
```

Os dois testes do host são o seam novo: um seletor registrado por `Host::set_file_picker` responde
por `workspace.pick` (inclusive `{ "path": null }` no cancelamento) e, sem hook, o host continua
devolvendo `pick_unavailable` fora do macOS. Os quatro de `src-tauri` cobrem o caminho do request
(`:1.42` → `1_42`), o token, a URI `file://` → caminho local e a assinatura `a(sa(us))` dos filtros.

As 3 falhas da suíte do host são `git::tests` (`log_reports_an_empty_repository_without_an_error`,
`update_leaves_a_conflicting_merge_for_the_user_to_resolve`, `update_reports_a_missing_identity_in_the_apps_own_words`)
e **não são desta mudança**: reproduzem em `eff5d21` sem ela. A causa é o locale pt_BR deste host —
`git log` num repositório vazio responde `seu branch atual 'master' ainda não tem nenhum commit` e o
código procura a mensagem em inglês; com `LANG=C` a mensagem é a esperada. O CI roda em inglês.

### App real (Linux, WebKitGTK)

As quatro capturas que provam o fluxo estão versionadas em `docs/evidence/screenshots/`:
`welcome-linux-04-picker-dialog.png`, `welcome-linux-05-picker-chosen-setup.png`,
`welcome-linux-06-picker-cancelled.png` e `welcome-linux-07-picker-unavailable.png`. O resto desta
sessão ficou em `/tmp/openridr-319-evidence/`; o texto abaixo é o OCR da janela.

1. **Abrir pelo botão.** Tela inicial fresca (a captura versionada é `welcome-linux-04-picker-dialog.png`): quatro `Tab` a partir do corpo levam o foco ao **Open** (o diff de pixels
   mostra o anel em `1815,102 137x64`, o canto superior direito) e `Enter` abre o diálogo do portal —
   janela `xdg-desktop-portal-gtk`, título `Open solution or project`, `875x600`. O combo de filtro
   mostra `Solutions and projects` e a lista de recentes. O OCR do diálogo:

   ```text
   Cancelar Open solution or project Q Selecionar
   Nome Localização Tamanho Tipo Acessado
   OrderDesk.sln ... icker-zbus/fixtures/OrderDesk 2,6 kB Texto 22:42
   worker-weather.sln ... s/polymarket-weather-worker 3,0 kB Texto 20:35
   ```

2. **Arquivo escolhido chegando ao setup.** Com o diálogo aberto, a célula `OrderDesk.sln` da tabela
   `Arquivos` foi focada e ativada pela árvore de acessibilidade (`grab_focus` + ação `activate`); o
   diálogo fechou e o app seguiu o fluxo normal de abertura. A tela (`welcome-linux-05-picker-chosen-setup.png`):

   ```text
   Setting up OrderDesk.sln
   Setup failed
   1 Analysis tool - csharp-ls 0.28.0  Verified  /tmp/openridr-319/tools/csharp-ls/0.28
   2 Debug adapter - netcoredbg 3.2.0-1092  Not available
   3 Analysis runtime - .NET 10  Found 10.0.14 at /home/helder/.dotnet
   ```

   O setup para no netcoredbg (`Not available`, o gate da #318), então a evidência termina aqui, como
   o brief permite. O log do app não tem `file chooser failed` nessa escolha: a resposta foi 0 com o
   caminho, não um erro.

3. **Cancelar.** `Ctrl+O` abriu o diálogo de novo (é também a evidência do
   atalho, que faz o mesmo que o botão) e o botão **Cancelar** foi acionado pela árvore de
   acessibilidade. O diálogo fechou, o log não ganhou nenhuma linha de erro (resposta 1 →
   `{ "path": null }`) e a tela ficou como estava: `29-welcome-before-cancel.png` e
   `welcome-linux-06-picker-cancelled.png` diferem em 141 pixels (0,0036%, o cursor de texto piscando), sem aviso e sem
   banner vermelho.

4. **Seletor indisponível, sem barramento de sessão.** O app subiu com
   `DBUS_SESSION_BUS_ADDRESS=unix:path=/tmp/no-such-bus-319`; `Ctrl+O` devolveu `pick_unavailable` e a
   tela (`welcome-linux-07-picker-unavailable.png`) mostrou a linha discreta e o campo com foco:

   ```text
   No native file chooser is available here; type the path instead.
   [Path to a .sln, .slnx, .csproj or folder (~/src/App.sln)
   ```

   O log: `openridr: file chooser unavailable: Failed to connect to address ... (os error 2)`.

5. **Seletor indisponível, sem backend de file chooser.** O app subiu num barramento privado
   (`dbus-run-session`) com `XDG_DESKTOP_PORTAL_DIR` apontando para uma pasta vazia: o portal ativa,
   mas não tem a interface `org.freedesktop.portal.FileChooser`. `Ctrl+O` recebeu
   `org.freedesktop.DBus.Error.UnknownMethod`, que o picker mapeia para `pick_unavailable`
   (sem captura versionada; a tela é a mesma da anterior), com a mesma linha discreta e o campo focado. O log:
   `openridr: file chooser unavailable: org.freedesktop.DBus.Error.UnknownMethod: Nenhuma interface
   "org.freedesktop.portal.FileChooser" no objeto ...`.

### Como o diálogo foi dirigido

Neste host o `wtype` digita texto no diálogo, mas não entrega `Return`/`Tab`/`BackSpace` ao GTK (o
`gtk_widget_event: assertion 'WIDGET_REALIZED_FOR_EVENT'` aparece no log do portal): o `Return` via
`hl.dsp.send_key_state` também não fecha o diálogo. A escolha do arquivo e o cancelamento foram feitos
pela árvore AT-SPI (`gi.repository.Atspi`), que expõe os botões `Selecionar`/`Cancelar` e a tabela
`Arquivos` com a célula `OrderDesk.sln`; `grab_focus` + `activate` na célula equivale ao duplo clique e
`do_action(0)` no botão equivale ao clique. Isso é ferramenta de evidência, não código do produto.

## Não verificado

- **macOS e Windows**: sem hardware. O macOS mantém `osascript` (o hook não é registrado) e o Windows
  mantém o campo de caminho; o CI `rust-tauri` roda no macOS, mas não abre o diálogo.
- **Conclusão do setup**: barrada pelo netcoredbg no Linux (#318); a escolha chega ao setup, que é o
  que esta frente podia medir.
- **Aprovação visual**: pendente do Helder; as capturas são a evidência, não um veredito.
- **Diálogo modal em relação à janela**: o `OpenFile` vai sem `parent_window` (o Tauri/tao não exporta
  um handle `xdg-foreign`), então no Hyprland ele é uma janela própria do portal.
- **`cargo clippy --workspace --all-targets -- -D warnings` no Linux**: vermelho por dead code
  pré-existente nos testes do host (`test_run.rs`, `test_discovery.rs`, `debugger.rs`, o import
  `hostroot`), fora desta mudança; o job `rust` roda no macOS. O clippy de `src-tauri` no Linux está
  verde depois do `allow(dead_code)` em `path_from_url` (que só é usada no macOS).
- **A suíte do host neste host**: 3 falhas de `git::tests` por locale pt_BR (acima), reproduzidas em
  `eff5d21`; o CI roda em inglês.
