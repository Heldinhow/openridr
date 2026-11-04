# Evidência — correções do E2E alpha.5

Branch `fix/e2e-alpha5`, 2026-11-04. Alvo: macOS 27.2 arm64, build de debug
`src-tauri/target/debug/bundle/macos/OpenRidr.app` (0.3.0-alpha.5), `OPENRIDR_HOME`
isolado em `<tmp>/verify-openridr-501-desktop/home`. `/Applications/OpenRidr.app` não foi
tocado.

Limite desta sessão: **Screen Recording continuou negado**, então não há capturas novas; a
prova da superfície C é snapshot de acessibilidade + estado em disco. Accessibility foi
concedida a tempo da maior parte do passe.

## P0

### BUG-001 — quit com buffer sujo (commit `ceaa473`)

Antes (relatório): Cmd+Q/menu/AppleScript encerravam sem diálogo.

Depois, no app real:

1. `Program.cs` aberto, `QZ9` digitado; aba "Program.cs, modified", "1 modified buffer".
2. `Cmd+Q` (`--force`) → diálogo **"Close OpenRidr with unsaved changes?"** com
   `Save all` / `Keep editing` / `Discard and close app`; processo 15204 vivo.
3. `Keep editing` → app vivo, buffer `QZ9` e "1 modified buffer" preservados.
4. `Cmd+Q` → `Save all` → `QZ9` no disco e processo encerrado.
5. Regressão do botão Close: com `DIRTY` no buffer, o Close da janela mostrou o mesmo
   diálogo; `Discard and close app` encerrou sem segundo diálogo.

Não coberto: Dock Quit e AppleScript `quit` usam `terminate:` nativo, que o Tauri não
expõe; registrado como limitação. `Cmd+Q`, menu Quit e o botão Close estão cobertos.

## P1

### BUG-002 — Debug após delete externo (commit `f4885ad`)

Antes: "1 saved source changed after this build (…Renamed.cs). Rebuild before debugging."

Depois, no app real: `Extra.cs` criado, Build verde, `Extra.cs` apagado por fora, Build
verde, Debug → sessão iniciou e imprimiu `Order A-104: R$ 42,00`; nenhuma mensagem citando
arquivo inexistente. O gate de hash foi corrigido na raiz (`crates/host/src/build.rs`) e o
teste de host cobre ausente-antes/ausente-depois, apagado-depois e modificado-depois.

## P2

### BUG-003 — buffer limpo (commit `d34a98e`)

No app real, `Program.cs` limpo e ativo; linha `// DRIFT1` prefixada por fora; o editor
mostrou o texto novo em ~3 s sem trocar de aba ou dar foco. e2e cobre aba limpa e conflito
de aba suja.

### BUG-004 — árvore (commit `403e330`)

No app real: `NewExternal.cs` criado, `Order.cs` → `Renamed.cs`, `OrderTests.cs` apagado
por fora; em ~4 s a árvore mostrou `NewExternal.cs` e `Renamed.cs` e removeu
`OrderTests.cs`, sem clicar em Re-evaluate.

### BUG-005 / BUG-006 — encoding (commit `e59bd4e`)

No app real:

- `Crlf.cs` (5 linhas CRLF) aberto, `X` digitado, `Save` → `grep -c $'\r'` = 5,
  `file` = "ASCII text, with CRLF line terminators".
- `Utf16.cs` (UTF-16 com BOM) aberto sem erro de UTF-8, `Y` digitado, `Save` → bytes
  `ff fe 59 00`, `file` = "Unicode text, UTF-16, little-endian"; conteúdo decodifica como
  `Y\uFEFFnamespace…`.

Host: round-trip CRLF/LF/misto, UTF-16LE/BE, UTF-8 com/sem BOM, hash de conflito estável.

### BUG-007 — `.cs`/`.txt` (commit `4c0dda5`)

O achado **não reproduzia no HEAD**: o relatório abriu a alpha.4 instalada via
`scripts/openridr` sem `OPENRIDR_APP`. Correções: o script valida extensão, imprime o
bundle e recusa `.cs`/`.txt`; e2e cobre `.cs`/`.txt` no app. No app real, abrir
`/tmp/Alpha5Single.cs` mostra "Open a .sln, a .slnx, a .csproj or a directory that contains
one." O `REPORT.md` foi corrigido.

### BUG-008 / BUG-015 — debugger (commits `a5680e7`, `d0f56c1`)

No app real, home nova: o clique em Debug baixou `netcoredbg 3.2.0-1092` e executou a
sessão; Workspace tools mostrou origem (`Samsung/netcoredbg`, release oficial), versão
`3.2.0` verificada com `--version` e SHA-256 do executável. O plano diz "experimental ·
disabled" com o motivo de flag de sessão, não "stays blocked on this platform".

O progresso do download de 3.4 MB não foi observado a olho (concluiu antes do snapshot);
o componente e o caminho de falha/retry são cobertos pelo teste de host com artefato
controlado e pelo e2e. O caminho do adaptador instalado passou a ser o do destino, não
`.staging` (achado da revisão, `d0f56c1`).

### BUG-009 … BUG-013, BUG-020 — skill (commit `0ce59af`)

Executado pela sessão do ticket:

- `doctor.sh` sem argumento avisa `app /Applications/OpenRidr.app 0.3.0-alpha.4 !=
  package.json`; com a cópia alpha.5 diz `ok … 0.3.0-alpha.5`; `OPENRIDR_APP` idem.
- `web.sh up/status/down`: 1520/4417, estado impresso, rotação para `.prev`, portas livres.
- `desktop.sh up <cópia>`: exatamente uma instância; `down` removeu o pid, exit 0,
  `pgrep` vazio. PID estrangeiro de `openridr` → exit 1, sem matar.
- `down` com PID de `sleep` gravado não mata processo alheio; remove o pid.
- Extração de accessibility sem JSON cru; screen recording também é reportado.
- Docs: `--headed` + `focus-window` para `type`/`press`; `debug-consent-dialog` e o
  "diálogo de troca" removidos; `verify-openridr` aponta para `test-app-agent-desktop`.

Limite: `desktop.sh up` ainda usa `/Applications` por padrão, mas agora imprime a versão e
avisa quando difere de `package.json`.

## P3

### BUG-014 — instância única (commit `f6cd2da`)

No app real (PID 17604): segunda execução direta do binário
`OPENRIDR_HOME=/tmp/alpha5-second-home openridr <sln>` saiu com código 0, `pgrep` continuou
com um único processo, e a instância existente abriu o Trust prompt do `.sln` encaminhado.
Dependência nova `tauri-plugin-single-instance 2.5.5` aprovada pelo Helder.

### BUG-016 / BUG-017 — mensagens (commit `4c0dda5`)

e2e: `.sln` sem projetos diz "lists no projects; not a valid solution" (sem SDK); TFM
inválido em `revision 1` não diz "previous evaluation"; `revision > 1` continua dizendo.
No app real, `Analysis runtime 10.0.14 · Project SDK 8.0.408` sem banner de pré-requisito.

### BUG-018 — acessibilidade (commit `24655a9`)

Snapshot do app real: `Save — File`, `Run — Tool Windows`, `Run — run.start`, menu de
projeto **"Project context menu"**. O menu de arquivo ainda se chama "Solution context
menu"; fora do escopo do achado (que citava o menu de projeto).

### BUG-019 — operações na árvore (commits `dfc0857`, `d0f56c1`)

No app real: New File → `UiCreated.cs` (0 bytes, árvore atualizada); Rename →
`UiRenamed.cs`; Delete → diálogo "Delete file?" → arquivo sumiu do disco e da árvore.
Confinamento e recusas (`..`, absoluto fora, pai inexistente, symlink) nos testes de host.
Revisão: o nó de projeto não oferece mais Rename/Delete, para não quebrar o `.sln`.

## Camada SDK/toolchain

- **Real app**: `Analysis runtime 10.0.14 · Project SDK 8.0.408`; projeto OrderDesk preso
  ao 8.0.408 avaliou, buildou, rodou e depurou sem pedir SDK 10 (`9ab2b62`, `ff297da`).
- **Descoberta**: DOTNET_ROOT, `/usr/local/share/dotnet`, `/opt/homebrew` (+libexec),
  `~/.dotnet`, SDKs gerenciados e todas as entradas do PATH, deduplicadas; testes de
  unidade e integração com raízes falsas (`cargo test -p openridr-host --lib` 213/213).
- **csharp-ls**: exige runtime `Microsoft.NETCore.App` 10, não SDK 10; plano continua
  revisável. Teste de host fez uma instalação real do NuGet (~99 MB, 6 s) com
  `downloading → installing → verifying → ready`; falha/retry com erro acionável
  (`3df4538`).
- **Links**: `dotnet.microsoft.com/download/dotnet` 200, `/download/dotnet/10.0` 200,
  `api.nuget.org/v3/index.json` 200, release do netcoredbg 200 (HEAD; nuget.org responde
  404 a HEAD e 200 a GET).
- **netcoredbg embutido**: proposta em `docs/PROPOSAL-NETCOREDBG-EMBED.md`, medida
  oficial 3.39 MB, decisão pendente do Helder; nenhum código de embed.

## Não verificado

- **Screenshots**: Screen Recording negado nesta sessão; prova por snapshot + disco.
- **Dock Quit / AppleScript quit**: caminho `terminate:` nativo, fora do hook do Tauri.
- **Progresso visual do csharp-ls e do netcoredbg no app**: componente e falha/retry
  cobertos por host/e2e; download real do csharp-ls só no teste de host.
- **Windows e macOS Intel**: fora de escopo.
- Itens do relatório marcados N/T (hover, rename LSP, breakpoints, Finder "Abrir com",
  múltiplas janelas) continuam não verificados; não são regressão desta rodada.

## Suítes executadas no HEAD final

- `bun run check` (tsc + `cargo check --workspace --all-targets`): ok.
- `cargo test -p openridr-host --lib`: 213/213; `--test debugger` 11/11; testes de host dos tickets (encoding, file ops, avaliação, SDK, tools) verdes.
- Playwright, 12 specs tocadas (workspace, medium-editor-state, solution-explorer, debugger-install-ux, debugger, open-external, evaluation, settings, trust-sdk, sdk-prerequisites, tools-install-ux, install-plan): **73 passed, exit 0** (3.5 min).
- `sh scripts/openridr-selfcheck.sh`: ok; `doctor.sh`, `web.sh up/status/down`, `desktop.sh up/down`: ok.
- Bundle de debug reconstruído após as correções de revisão e revalidado no app real (menu de projeto sem Rename/Delete, caminho do adaptador no destino, versão impressa pelo `desktop.sh`).
- Correções da revisão: `d0f56c1`, `226a3aa` (erros dos listeners do shell e rechecagem antes do save).
