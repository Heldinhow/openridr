# Evidência — Abrir com (tickets 02–03)

Escopo: [SPEC-OPEN-WITH](../SPEC-OPEN-WITH.md), tickets [02](../tickets-open-with/02-tauri-open-request.md)
e [03](../tickets-open-with/03-launcher-docs.md). Data: 2026-11-02, macOS 27.2 arm64.

## Bundle de debug

- Caminho: `src-tauri/target/debug/bundle/macos/OpenRidr.app`.
- `Info.plist`: `CFBundleIdentifier` = `com.openridr.desktop`.
- `CFBundleDocumentTypes`: 4 entradas — `sln`, `slnx`, `csproj` (Editor, `LSHandlerRank` Default) e
  `public.folder` (Editor, `LSHandlerRank` Alternate).
- `UTExportedTypeDeclarations`: `com.openridr.solution`, `com.openridr.solution-xml`,
  `com.openridr.csproj`.

## LaunchServices

- `lsregister -dump`: o bundle reclama as UTIs `com.openridr.solution`, `com.openridr.solution-xml`,
  `com.openridr.csproj` e `public.folder`.
- `mdls` nos arquivos reais depois do registro: `OrderDesk.sln` = `com.openridr.solution`,
  `OrderDesk.Cli.csproj` = `com.openridr.csproj` e um `Modern.slnx` temporário =
  `com.openridr.solution-xml`. É o que faz o Finder oferecer o app em **Abrir Com**.

## Aberturas com `open -a` (teste nativo isolado)

Nenhuma instância do app do Helder foi tocada. Procedimento:

1. Cópia do bundle em diretório temporário, com `CFBundleIdentifier` trocado para
   `com.openridr.verify.openwith` e assinatura ad-hoc.
2. `OPENRIDR_HOME=$PWD/.scratch/host-home` com `trusted.json` isolado.
3. Comandos: `open --env OPENRIDR_HOME=… -a <cópia> <alvo>`.

| Caso | Alvo | Estado inicial | Resultado |
|---|---|---|---|
| Arquivo com app fechado | `fixtures/OrderDesk/OrderDesk.sln` | fechado | `.sln` gravado em `recent.json`; 1 processo |
| Arquivo com app aberto | `fixtures/OrderDesk/OrderDesk.Cli.csproj` | aberto | `.csproj` gravado em `recent.json`; continua 1 processo |
| Pasta | `fixtures/OrderDesk` | aberto | resolvido para `OrderDesk.sln`; continua 1 processo |

- `~/Library/Application Support/OpenRidr` não ganhou arquivos: o mtime do diretório é anterior ao
  teste (21:10).

## Launcher `scripts/openridr`

- `sh -n scripts/openridr`: ok.
- Self-check commitado: `sh scripts/openridr-selfcheck.sh` (`open` falso num PATH temporário, removido
  no `trap`). Cobre argv `-a OpenRidr <caminho>`, `OPENRIDR_APP` com espaços, zero e dois argumentos
  (saem ≠ 0 sem chamar `open`), caminho inexistente (saída ≠ 0, mensagem em `stderr`, `open` não
  chamado). Resultado: `ok: openridr self-check passed`.
- Não rodado contra o bundle real: o launcher não repassa `OPENRIDR_HOME`, e rodá-lo sem isolamento
  gravaria no app home do usuário.

## Não verificado

- Botões de candidato (`external-open-candidate-N`, com workspace montado): implementados, mas a UI não foi exercitada
  nem visualmente nem por E2E. O host já provou a recusa ambígua (`open-external.spec.ts`).
- Recusa de abertura externa com prompt de Trust pendente: implementada em `App.openExternal`, sem prova na tela.
- Prompt de Trust e banner de erro na tela (visual).
- Recusa por buffer sujo em abertura externa.
- Escolha de candidato na tela inicial depois de um diretório ambíguo vindo do Finder/Dock: o caminho
  foi roteado para `SetupScreen` (que mostra a lista) e o `workspace.open` ambíguo foi provado no host;
  a lista na tela e o clique não têm E2E de UI.
- Duplo clique no Finder e menu **Abrir Com** visual. As claims do LaunchServices são a evidência.
- Arrasto real de pasta para o Dock: usei `open -a` com pasta como proxy.
- Launcher contra o bundle real.
- `.slnx` pelo Finder/LaunchServices: não foi aberto no bundle. A abertura e o outline de `.slnx`
  estão cobertos só no host (E2E do ticket 01). Restore e build não funcionam com SDK 8.0.408.
- Windows, macOS Intel, notarização e assinatura de distribuição: fora do escopo.
