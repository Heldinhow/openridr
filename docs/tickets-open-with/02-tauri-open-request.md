# 02 — Bundle e recepção no shell Tauri

**Fonte:** `docs/SPEC-OPEN-WITH.md` §Implementation Decisions.
**Bloqueado por:** 01. **Status:** ready-for-agent.

## Objetivo

O bundle declara `.sln`, `.slnx`, `.csproj` e pastas; abrir um deles pelo Finder (app fechado
ou aberto) ou arrastar uma pasta para o Dock entrega o caminho à UI, que o abre pelo fluxo
existente — sem segunda janela, sem execução e sem perder buffer sujo.

## Pontos de código

- `src-tauri/tauri.conf.json` → `bundle.fileAssociations`:
  - `.sln`: `name` "Solution", `role` "Editor", `rank` "Default", `mimeType` "text/plain",
    `exportedType.identifier` `com.openridr.solution`, `conformsTo` `["public.plain-text"]`.
  - `.slnx`: `com.openridr.solution-xml`, `conformsTo` `["public.xml"]`, mime XML.
  - `.csproj`: `com.openridr.csproj`, `conformsTo` `["public.xml"]`, mime XML.
  - Pasta: `ext: []`, `name` "Folder", `role` "Editor", `rank` "Alternate",
    `contentTypes: ["public.folder"]`. Se o bundler recusar `ext` vazio, usar
    `src-tauri/Info.plist` **sem** apagar as entries geradas e registrar a alternativa.
- `src-tauri/src/main.rs`:
  - `first_file_argument(args)` ignora flags (inclui `-psn_...`) e devolve o primeiro caminho.
  - `path_from_url(url)` só aceita `file://` e usa `to_file_path`.
  - `PendingOpen(Mutex<Option<String>>)` no estado Tauri; comando `take_pending_open`.
  - `setup` lê o argv; `App::run` trata `RunEvent::Opened` (guard `#[cfg(target_os = "macos")]`
    para os demais alvos compilarem). Guardar o último caminho e emitir `openridr-open`.
  - Testes `#[cfg(test)]` para argv e URL; rodar
    `cargo test --manifest-path src-tauri/Cargo.toml`.
- `src/App.tsx`:
  - Ao montar e ao receber `openridr-open`, chamar `take_pending_open` (só em Tauri) e abrir
    com `workspace.open`, reusando `handleOpened`.
  - Workspace já montado: encaminhar o caminho ao `WorkspaceView`.
  - Falha de abertura: faixa `role="alert"` com `data-testid="external-open-error"`,
    dispensável, sem mudar o workspace atual.
- `src/WorkspaceView.tsx`: prop de abertura externa consumida por um efeito que chama o
  `openSolution` existente (guard de `dirtyCount` e mensagem atuais). Não duplicar a regra.
- `src/styles.css`: estilo da faixa de erro externo, reusando `.notice.error` e
  `.notice-dismiss`.

## Teste primeiro

1. `cargo test --manifest-path src-tauri/Cargo.toml`: argv ignora `-psn_0_1`, caminho com
   espaços é aceito, `file:///tmp/A.sln` vira caminho, `https://...` é ignorado.
2. `bun run build` (tsc) antes e depois da UI.
3. Build do bundle: `bun run tauri build --debug --bundles app` e conferir no Info.plist gerado
   `CFBundleDocumentTypes` com `.sln`, `.slnx`, `.csproj` e `public.folder`, e
   `UTExportedTypeDeclarations` com os três identificadores.
4. Manual com dados isolados (não tocar em `~/Library/Application Support/OpenRidr`):
   - `open --env OPENRIDR_HOME=$PWD/.scratch/host-home --env OPENRIDR_TRUST_STORE=off -a <bundle> fixtures/OrderDesk/OrderDesk.sln`
     com o app fechado: uma janela, prompt de Trust do OrderDesk.
   - repetir com o app aberto: continua uma janela e o workspace troca.
   - `open --env ... -a <bundle> fixtures/OrderDesk` (pasta): mesmo efeito.
   - fechar o app e conferir que nenhum arquivo foi criado em `~/Library/Application Support/OpenRidr`.
   Registrar o que funcionou e o que não foi possível no `docs/implementation-notes.md`.

## Aceite

- [x] Associações no bundle gerado (`.sln`, `.slnx`, `.csproj`, pasta).
- [ ] argv, `Opened` e URL inválida tratados; último caminho pendente vence.
- [ ] UI abre o caminho pelo mesmo `workspace.open`; trust/recent inalterados.
- [ ] Workspace aberto com buffer sujo recusa a abertura externa com a mensagem existente.
- [ ] Uma janela só; nenhum processo/dotnet ao abrir.
- [x] Unidades do shell, `bun run build`, `cargo check` e o build do bundle verdes.
- [x] Nenhum arquivo em `~/Library/Application Support/OpenRidr` ao testar.
- [x] Commit local `feat(app): open solutions from Finder and the Dock` só com os caminhos
      deste ticket.
