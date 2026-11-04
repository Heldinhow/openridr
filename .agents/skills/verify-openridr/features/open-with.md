# 4. Abrir pelo Finder / Dock / terminal (`scripts/openridr`)

## Sub-features

- `scripts/openridr <caminho>`: valida uso/caminho/extensão (pasta ou `.sln`/`.slnx`/`.csproj`), imprime o bundle efetivo e chama `open -a "${OPENRIDR_APP:-OpenRidr}" <caminho>` (reusa a instância aberta).
- "Abrir com" do Finder para `.sln`/`.slnx`/`.csproj` e pastas; arrastar pasta para o Dock.
- No shell Tauri (`src-tauri/src/main.rs`): argv (`first_file_argument`, ignora `-psn_*`) e `RunEvent::Opened` (`path_from_url`, só `file://`) viram `PendingOpen` (último vence) + evento `openridr-open`; a UI chama `take_pending_open` (`src/App.tsx`, `openExternal`).
- UI: com workspace aberto o caminho novo **substitui** o workspace direto (`WorkspaceView.tsx` chama `openSolution`; não existe diálogo de troca — o prompt de Trust pode aparecer em seguida se o projeto ainda não foi decidido); sem workspace → setup/Trust; com prompt de Trust pendente → recusa com `external-open-error`; duas solutions na pasta → `external-open-candidates`.

## How to get to it (user POV)

`openridr ~/proj/App.sln` no terminal, Finder → Abrir com → OpenRidr, ou arrastar a pasta para o Dock.

## Driving it

- **Script** (sem GUI, não chama o `open` real): `sh scripts/openridr-selfcheck.sh` → `ok: openridr self-check passed`.
- **Shell Rust**: `cargo test --manifest-path src-tauri/Cargo.toml` (argv e URL).
- **A** (resolução no host, que é o que a UI só repassa): `bunx playwright test tests/e2e/open-external.spec.ts`.
- **C** (único jeito de provar a entrega até a UI): `desktop.sh up` (argv = mesmo caminho do CLI) e, com o app já aberto, `OPENRIDR_APP=~/Applications-test/OpenRidr-alpha5/OpenRidr.app sh scripts/openridr "<state>/work/OrderDesk/OrderDesk.sln"` (use o `<state>` impresso pelo `up`; não aponte para `/Applications`, que nesta máquina é a alpha.4); então `agent-desktop snapshot --app OpenRidr -i --compact` deve mostrar o diálogo/prompt para o caminho. Requer tela desbloqueada e Acessibilidade.

Prova de que funciona: o caminho passado aparece como workspace/prompt na UI **e** `trusted.json`/recentes do home isolado refletem a decisão tomada.

## Gotchas

- `open -a` para o bundle com o mesmo `CFBundleIdentifier` (`com.openridr.desktop`) pode entregar ao app **do usuário**, com o home real dele. Só dirija C com o `desktop.sh up` (home isolado) e nunca com outra instância aberta.
- Um app lançado pelo LaunchServices não herda `OPENRIDR_HOME`; por isso o `desktop.sh` roda o binário direto.
- As associações vêm do `Info.plist` gerado: mudou `tauri.conf.json` → rebuild do `.app` antes de provar.
- O app não é notarizado (Gatekeeper pede botão direito → Abrir na primeira vez).
- **Estado em 2026-11-03**: C provada com `~/Applications-test/OpenRidr-alpha5/OpenRidr.app` (Acessibilidade concedida): argv e `open -a` entregaram o caminho e o Trust foi reutilizado. Ver `docs/evidence/e2e-alpha5/`.
