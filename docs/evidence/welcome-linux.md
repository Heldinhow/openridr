# Welcome no Linux: Open, campo de caminho e plataforma (#319)

Máquina: Arch Linux ARM, Wayland/Hyprland 0.57.1, `xdg-desktop-portal` 1.22.1 com os backends `gtk`
1.15.3 e `hyprland` 1.5.0, `~/.dotnet` com 8.0.408 e 10.0.403. Branch `fix/issue-319-welcome-linux`
sobre `9564e1b`. O app rodou **só** pelo binário desta worktree
(`target/debug/openridr`, via `bunx tauri dev`), com `OPENRIDR_HOME=/tmp/openridr-319` e o dev server
na porta 2420; a janela foi identificada pelo PID e levada para o workspace 8. Nada foi instalado em
`$HOME` (isso é da #318).

## O que foi verificado

### Testes automatizados

```sh
$ cargo test -p openridr-host --lib -- tilde
running 1 test
test tests::a_tilde_prefix_expands_against_home_and_nothing_else_is_rewritten ... ok

$ cargo test -p openridr-host --test host -- file_chooser
running 1 test
test the_file_chooser_refuses_where_no_native_dialog_exists ... ok

$ bun test tests/unit/welcomeOpen.test.ts
 4 pass, 0 fail

$ PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
  DOTNET_CLI_UI_LANGUAGE=en PLAYWRIGHT_CHANNEL=chromium \
  bunx playwright test tests/e2e/welcome.spec.ts --grep-invert "keyboard"
  8 passed (7.9s)
```

Os 8 verdes incluem os três casos novos: a dica segue a plataforma (`Ctrl+O`, nunca `⌘O`), o caminho
digitado abre o projeto e um caminho errado mantém o texto com o erro ao lado do campo, e um seletor
recusado entrega o foco ao campo como linha de status, sem `workspace-error`.

**`welcome.spec.ts › keyboard` fica vermelho no Linux e não é desta frente**: ele abre um projeto
confiável e espera a árvore, mas o gate de setup barra no netcoredbg ("Not available", #318 — o
`.tar.gz` do adaptador não é extraído neste build). O erro é o mesmo antes desta mudança; o resto do
teste (↑/↓, `/`, `Ctrl+O` pedindo o seletor) passou até ali.

### App real (Linux, WebKitGTK)

Capturas desta sessão em `/tmp/openridr-319-evidence/welcome-linux-*.png` (não versionadas: `docs/evidence/screenshots/` só guarda arquivo já rastreado, ver `.gitignore`); o texto abaixo é o OCR da janela.

1. `welcome-linux-01-projects.png` — tela inicial sem barra de menu GTK (a primeira linha é o
   cabeçalho do app, não `Edit Window Help`), com o campo de caminho visível e a dica
   `Ctrl+O open project`:

   ```text
   </> Welcome to OpenRidr
   Q Search projects
   Path to a .sln, .slnx, .csproj or folder (~/src/App.sln)
   No recent projects yet
   ↑ ↓ select   Enter open   / search   Ctrl+O open project
   ```

2. `welcome-linux-02-chooser-unavailable.png` — `Ctrl+O` no app real: o host responde
   `pick_unavailable` e a tela mostra a linha discreta, sem erro vermelho, com o campo focado (o
   cursor aparece no campo e a digitação seguinte entra nele):

   ```text
   No native file chooser is available here; type the path instead.
   ```

3. `welcome-linux-03-typed-path-setup.png` — caminho digitado no campo + Enter chega ao fluxo normal
   de abertura, que nesta branch termina na tela de setup (o gate da #318):

   ```text
   Setting up OrderDesk.sln
   Setup failed
   1 Analysis tool - csharp-ls 0.28.0  Verified  /tmp/openridr-319/tools/csharp-ls/0.28
   2 Debug adapter - netcoredbg 3.2.0-1092  Not available
   3 Analysis runtime - .NET 10  Found 10.0.14 at /home/helder/.dotnet
   ```

   Também verificado no app real: um caminho inválido mostra `Workspace path not found: <o que foi
   digitado>` logo abaixo do campo e o texto digitado continua lá.

## Por que o seletor nativo ainda não existe

O caminho sem dependência nova era o portal XDG por `gdbus`. Ele não fecha:

```sh
$ (gdbus monitor --session --dest org.freedesktop.portal.Desktop > /tmp/mon.log 2>&1 &)
$ gdbus call --session --dest org.freedesktop.portal.Desktop \
    --object-path /org/freedesktop/portal/desktop \
    --method org.freedesktop.portal.Screenshot.Screenshot "" \
    '{"handle_token": <"openridr_probe_1">, "interactive": <false>}'
(objectpath '/org/freedesktop/portal/desktop/request/1_749/openridr_probe_1',)
$ cat /tmp/mon.log
Monitoring signals from all objects owned by org.freedesktop.portal.Desktop
The name org.freedesktop.portal.Desktop is owned by :1.24
```

Nenhum `Response` no monitor: o portal emite a resposta **unicast** para a conexão que chamou
(`request->sender`), então um segundo processo não a vê. E o pedido é amarrado ao chamador:

```sh
$ gdbus call --session --dest org.freedesktop.portal.Desktop \
    --object-path /org/freedesktop/portal/desktop/request/1_763/openridr_probe_8 \
    --method org.freedesktop.portal.Request.Close
Erro: GDBus.Error:org.freedesktop.DBus.Error.AccessDenied: Portal operation not allowed: Unmatched caller
```

Ou seja: `gdbus call` sai, a conexão do chamador morre e o pedido vai com ela — não dá para chamar num
processo e esperar a resposta em outro. Um cliente de uma conexão só exige uma biblioteca D-Bus
(`zbus`, que já está em `src-tauri/Cargo.lock` como transitiva, mas passaria a ser dependência
direta) ou um runtime do sistema com GIO (`gjs`, instalado nesta máquina só por causa do `sushi`).
Decisão de dependência pendente com o Helder; até lá o Linux usa o campo de caminho, que é a saída
que a issue exige.

## Não verificado

- **Seletor nativo no Linux**: não existe ainda (dependência pendente). Por isso "arquivo escolhido
  chegando à tela de setup" e "cancelamento do seletor" não foram medidos no Linux; o caminho
  digitado foi.
- **Recortar/copiar/colar/selecionar tudo nos campos**: `Ctrl+A` e `Ctrl+V` não responderam no
  WebKitGTK desta sessão **com e sem** a barra de menu (testado nas duas configurações, com
  `wl-copy`/`wl-paste` provando que a área de transferência tinha o texto). O menu não era o dono
  dessas teclas (ver `docs/decisions.md`, entrada de 2026-11-08), então a remoção não as tira; o que
  responde por elas é o WebKitGTK. Falta a confirmação com teclado real.
- **macOS e Windows**: sem hardware. O macOS mantém `osascript`, o menu nativo, `⌘O` e o campo
  escondido (coberto por `tests/unit/welcomeOpen.test.ts`); o Windows ganha o campo por não ser
  macOS e mantém `Menu::default`.
- **Aprovação visual**: pendente do Helder. As capturas acima são a evidência, não um veredito de
  aparência; elas ficam em `/tmp/openridr-319-evidence/` porque o repositório ignora captura nova.
