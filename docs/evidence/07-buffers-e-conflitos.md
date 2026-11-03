# Evidência — Ticket 07: buffers e conflitos de disco

Implementação local em 28/10/2026, base `8aee0e2`. Fonte normativa: [ticket 07](../tickets/07-buffers-e-conflitos.md), [SPEC](../SPEC.md) AC-11/12/13/19/32/33 e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Bun / Node | 1.3.9 / v26.10.0 |
| Rust | cargo 1.99.0, rustc 1.99.0 |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (detectado; não instalado pelo app) |
| csharp-ls | 0.28.0 em `~/Library/Application Support/OpenRidr/tools/csharp-ls/0.28` |
| Fixture | `fixtures/OrderDesk`; `Program.cs` e `Order.cs` restaurados em `beforeEach`/`afterEach` |
| Fixture dos testes de buffer | modo restrito, sem trust, sem MSBuild, sem LSP |

Log: `logs/07-environment.log` *(evidência removida do repositório público)*.

## O que foi construído

- **Buffer com revisões** (`src/WorkspaceView.tsx`): cada aba guarda `content`, `hash` (hash base conhecido), `revision` (0 no open, +1 por mudança de conteúdo), `savedRevision` e `conflict`. “Modified” é `revision !== savedRevision || conflict`, não comparação de texto; por isso editar e desfazer não declara limpo, e recarregar/sobrescrever/salvar mantêm aba, marcador, hash e revisão coerentes. O breadcrumb mostra `revision N · saved M · modified|saved` (`editor-revision`).
- **Recarga não é edição** (`src/CodeEditor.tsx`): a aplicação externa do valor (reload) não dispara `onChange`; o reload atualiza conteúdo/hash/revisão uma única vez e envia `didChange` ao LSP com o texto novo.
- **Fechamento com decisão explícita**: o diálogo de aba tem `Save`, `Keep editing` e `Discard and close`; o de workspace tem `Save all`, `Keep editing` e `Discard and close workspace`; o de aplicação (Tauri) tem `Save all`, `Keep editing` e `Discard and close app`. Falha de escrita ou conflito novo mantém o buffer e o diálogo abertos com o erro em `confirm-error`; `Keep editing` não altera nada. O save de aba usa o buffer daquela aba, e o save all percorre as abas sujas em sequência.
- **Fechamento do app**: `beforeunload` é cancelado somente quando existe buffer sujo/conflito (Chrome/E2E); no Tauri, `onCloseRequested` cancela o fechamento nativo e mostra o mesmo diálogo; o descarte chama `destroy()` com a permissão mínima `core:window:allow-destroy`. Nenhum fechamento silencioso com edição pendente.
- **Comparação de conflito**: o banner ganhou `<details>` com painéis lado a lado “Buffer · unsaved · revision N” (hash base) e “Disk · external change” (hash do disco), conteúdo integral em `<pre>` e contagem concreta de linhas removidas/adicionadas (LCS por linha; acima de 1200 linhas mostra apenas os hashes). `Reload from disk (discard my edits)` e `Overwrite disk` continuam explícitos.
- **Revalidação antes de sobrescrever**: `Overwrite disk` usa o hash da versão em comparação como `expected_hash`; se o disco mudou de novo, o host devolve `conflict`, o banner atualiza a versão do disco e nada é gravado. O E2E força duas versões externas e prova que a segunda gravação não acontece.
- **Detecção proativa**: ao ganhar foco e ao ativar uma aba, a UI relê o arquivo (`file.read`). Hash diferente do hash base/conflito entra em conflito com o conteúdo do disco, sem substituir o buffer; hash igual não altera nada. Leituras em voo são invalidadas por uma época por path e ignoradas durante gravações. Falha de leitura mostra erro não destrutivo e o buffer continua editável. Tudo isso funciona em modo restrito.
- **Processos preservados**: fechar workspace com buffer sujo usa o mesmo `workspace.close`; o teste real com processo `sleep` rastreado (ticket 02) agora fecha com buffer sujo, descarta explicitamente e reafirma `Processes`, `awaiting decision` e `Terminate` na tela de setup. Nenhuma semântica de término foi alterada.

## Comandos e saídas reais

### Frontend

```text
$ bun run build
tsc --noEmit && vite build
✓ 58 modules transformed.
dist/assets/index-qZ764Ykl.js   634.74 kB │ gzip: 203.17 kB
✓ built in 1.38s
```

Log: `logs/07-build.log` *(evidência removida do repositório público)*. O aviso de chunk >500 kB é do CodeMirror e não é erro.

### Host (inalterado; unit + boundary)

```text
$ cargo test -p openridr-host
running 39 tests
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 40 tests
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
Doc-tests openridr_host: 0 passed
```

Log: `logs/07-cargo-test.log` *(evidência removida do repositório público)*. Nenhum código de host mudou neste ticket, então nenhum teste de boundary novo foi necessário; os testes existentes de path/UTF-8/read-only seguem verdes.

### Playwright (Chrome real + host real + arquivos reais)

```text
$ bun run test:e2e
  ✓ saves a modified buffer from the close dialog (945ms)
  ✓ a save failure from the close dialog keeps the buffer and explains the error (983ms)
  ✓ saves every modified buffer from the workspace close dialog (1.2s)
  ✓ compares buffer and disk and revalidates a second disk change before overwriting (1.5s)
  ✓ detects external changes on focus and tab activation without replacing the buffer (1.1s)
  ✓ prevents beforeunload only while a buffer is dirty (700ms)
  ✓ revoked workspace shows the remaining process and terminate observes the exit (2.4s)
  26 passed (1.4m)
```

Log completo: `logs/07-playwright.log` *(evidência removida do repositório público)*. A suíte inclui os 20 testes anteriores (build/restore/evaluation/csharp-ls/trust) e 6 novos de buffer; `workers: 1` porque host e arquivos são estado compartilhado.

### Tauri (compile check)

```text
$ bun run tauri build --debug --no-bundle
   Compiling openridr v0.1.0 (~/Projetos/openridr/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.44s
       Built application at: ~/Projetos/openridr/src-tauri/target/debug/openridr
```

Log: `logs/07-tauri-build.log` *(evidência removida do repositório público)*. A permissão `core:window:allow-destroy` foi aceita pelo schema/capability.

## Comparação de conflito (UI real)

[`screenshots/07-conflict-compare.png`](screenshots/07-conflict-compare.png) — captura Playwright do build servido ao Chrome real, viewport 1440×900: breadcrumb `revision 1 · saved 0 · modified`, banner com os dois painéis, hash base e hash do disco (`fnv1a64:…`), contagem `1 line removed · 1 line added`, e as ações Reload/Overwrite.

Não há screenshot da janela Tauri: a captura de tela do desktop continua bloqueada por TCC nesta máquina (ver ticket 01). A imagem é da mesma UI/host servidos ao Chrome, não uma prova de interação com a janela nativa.

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| Fechar aba/app/workspace com edição oferece salvar, preservar/cancelar e descarte explícito; cancelar não altera nada; falha ao salvar mantém conteúdo e explica | Aba e workspace: `Save`/`Save all`, `Keep editing`, `Discard` provados por E2E; falha de escrita mantém buffer, diálogo e `confirm-error`; descarte só no botão. App: `beforeunload` provado no Chrome; `onCloseRequested` implementado e compilado, janela nativa não dirigida. | E2E “saves a modified buffer…”, “a save failure…”, “saves every modified buffer…”, “prevents beforeunload…” |
| Detectar edição externa antes de sobrescrever; comparar, recarregar por decisão ou preservar edições; não substituir silenciosamente | Foco e ativação de aba entram em conflito sem trocar o buffer; compare mostra buffer/disco com hashes e diferença; reload só no botão; Overwrite revalida e conflita de novo na segunda mudança. | E2E “compares buffer and disk…”, “detects external changes…” + screenshot |
| Abas/breadcrumb/modificado/revisão coerentes após salvar, recarregar, preservar ou cancelar; mudanças de disco invalidam snapshot antigo | `editor-revision` e dirty derivam de `revision`/`savedRevision`; após reload e overwrite, um novo save usa o hash gravado e não conflita; conflito proativo invalida a base antiga. Edições semânticas (09/10) ainda não existem; o contrato é passar pelo mesmo `file.write`. | E2E de revisão e de hash pós-reload/pós-overwrite |
| Proteção sem solução/SDK/LSP e com falha do host; concorrência durante confirmação é revalidada; edições semânticas futuras não contornam | Toda a suíte nova roda em modo restrito; leitura removida no meio do teste mantém buffer editável com erro; escrita sem permissão mantém buffer; segunda mudança de disco durante conflito não grava. | E2E “detects external changes…”, “a save failure…”, “compares buffer and disk…” |
| Troca/fechamento não oculta processos remanescentes; G1–G5 na superfície disponível | Fechamento com buffer sujo + processo `sleep` real: setup continua mostrando `Processes`, `awaiting decision` e término observado; G1 não muda (nenhum método executável novo); G2 hash/revisão/contexto; G4 labels/keyboard/foco; G5 logs reais. | E2E “revoked workspace shows…”; esta página |
| G2 — integridade | Hash FNV-1a do texto, `expected_hash` obrigatório, conflito devolve `currentContent`/`currentHash`; overwrite revalida o hash comparado; nenhum caminho de escrita ignora o hash. | `cargo-test`, E2E de conflito |
| G4 — UI/privacidade | Strings em inglês, `<details>` acessível por teclado, labels `Buffer version`/`Disk version`, foco visível nos botões, sem telemetria. | screenshot + E2E |

ACs da SPEC tocados: **AC-11** (ciclo completo de buffers/conflitos em modo restrito), **AC-12/13 parcial** (proteção/revisão para os consumidores 09/10), **AC-19 parcial** (preservação de edições ao fechar; build/run em 11/12), **AC-32**, **AC-33 parcial** (macOS arm64).

## Limitações e pendências honestas

- **Janela nativa não dirigida**: o diálogo de fechamento Tauri (`onCloseRequested` + `destroy()`) compila e a capability foi aceita, mas não houve clique real na janela nativa; TCC bloqueia screenshot/System Events nesta máquina. `beforeunload` foi provado no Chrome. Um smoke humano da janela nativa continua recomendado antes de release.
- **Windows x64 e macOS Intel não executados**: CRLF, paths e permissões Windows não foram exercitados; a fixture e os testes rodam em macOS arm64. Fica como pendência de G5/18.
- **Diff é contagem, não marcação linha a linha**, e é omitido acima de 1200 linhas por lado (comentário `shortcut:` no código); os hashes continuam disponíveis.
- **Edições semânticas futuras** (tickets 09/10) ainda não existem; este ticket fixa o contrato de `expected_hash` e de revisão que elas devem usar.
- **Nenhum código de host mudou**; a proteção de path/UTF-8/read-only é a do ticket 01 e continua coberta pelos 39+40 testes e pelos E2E anteriores.
- **Revalidação não é lock de SO**: o host compara `expected_hash` com o disco no início do `file.write` e grava por temp+rename. Uma escrita externa entre essa checagem e o rename não é detectável por hash; o ticket não altera o host nem promete atomicidade contra outro processo.
