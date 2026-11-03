# Evidência — Ticket 18: aceitação multiplataforma e auditoria de release

Implementação local em 28/10/2026, sobre `4b8885c`, em um único commit
`feat(acceptance)` (docs + testes + pequenas correções encontradas na auditoria).
Fonte normativa: [ticket 18](tickets/18-aceitacao-e-release.md), [SPEC](SPEC.md),
[índice/G1–G5](tickets/README.md), [matriz](acceptance-matrix.md) e
[decisão de release](release-decision.md). Legenda: `E01…E17` = `docs/evidence/NN-*.md`;
`A18` = este documento; `L18-*` = `docs/evidence/logs/18-*`; `S18-*` =
`docs/evidence/screenshots/18-*`.

**Resultado: release NÃO aprovado.** Mac ARM64 tem evidência real do núcleo P0; Windows x64 e
macOS Intel não foram executados e permanecem gates abertos. Nenhuma aprovação foi inventada.

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), Apple M2, arm64; Xcode 27.1 |
| Shell / ferramentas | Bun 1.3.9; Node v26.10.0; cargo/rustc 1.99.0; Chrome 155.0.8059.85 (canal `chrome`); Playwright 1.64.1 |
| Host de análise | .NET SDK 10.0.100 em `/usr/local/share/dotnet` (também 8.0.416) |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`; `global.json` `rollForward: disable` |
| Runtimes | `Microsoft.NETCore.App` 8.0.22 e 10.0.0 em `/usr/local/share/dotnet/shared` |
| csharp-ls | 0.28.0 (`csharp-ls, 0.28.0 (Klebiškis)+72671fc7…`), instalado via NuGet em app home isolado de E2E |
| netcoredbg | 3.2.0-1092 osx-arm64 (`NET Core debugger 3.2.0-1 (9744e1f, Release)`), asset oficial Samsung |
| Fixture | `fixtures/OrderDesk` (console + biblioteca + testes xUnit), net8.0; sempre copiada para temp com espaço no nome |
| Base anterior | `4b8885c`; nenhum remote/push/PR |
| App home de E2E | `$TMPDIR/openridr-e2e-home`; o app home real `~/Library/Application Support/OpenRidr` não foi apagado |

Log de ambiente: `logs/18-environment.log` *(evidência removida do repositório público)*.

## Comandos e resultados reais

| Comando | Resultado | Log |
|---|---|---|
| `bun run build` | `tsc --noEmit` + Vite; 62 módulos; aviso de chunk >500 kB; exit 0 | `logs/18-build.log` *(evidência removida do repositório público)* |
| `cargo test -p openridr-host` | 192 testes passaram, 0 falharam (soma dos binários; doc-tests 0) | `logs/18-cargo-test.log` *(evidência removida do repositório público)* |
| `bun run test:e2e` | 49 testes passaram, incluindo o spec de aceitação | `logs/18-playwright-full.log` *(evidência removida do repositório público)* |
| `bun run test:e2e tests/e2e/zz-acceptance.spec.ts` | 2 testes passaram (jornada + auditoria de UI/layout) | `logs/18-playwright-acceptance.log` *(evidência removida do repositório público)* |
| `bun run tauri build --debug --no-bundle` | app em `src-tauri/target/debug/openridr`; exit 0 | `logs/18-tauri-build.log` *(evidência removida do repositório público)* |
| `cargo metadata` + `cargo tree --target aarch64-apple-darwin` | licenças/grafo do app; 232 nomes de crates no grafo macOS | `logs/18-licenses-summary.txt` *(evidência removida do repositório público)*, `logs/18-licenses-app-graph.tsv` *(evidência removida do repositório público)* |
| scan de `node_modules`/`bun.lock` | 157 entradas no lock; 96 pacotes instalados; licenças MIT/ISC/Apache/BSD/CC-BY-4.0 | `logs/18-licenses-npm.tsv` *(evidência removida do repositório público)* |

O spec de aceitação roda por último (`zz-acceptance.spec.ts`) porque as suítes por ticket
assumem sessão/sequência inicial do host; ele não substitui nenhuma delas. A ordem é uma dívida
de teste registrada na decisão de release.

## Jornada real integrada (macOS ARM64)

Spec: [`tests/e2e/zz-acceptance.spec.ts`](../tests/e2e/zz-acceptance.spec.ts). UI real no Chrome
contra Vite + `openridr-host-http` (mesmo `dispatch` do app); ferramentas reais; viewport 1440×900.
Resumo em `logs/18-acceptance-summary.json` *(evidência removida do repositório público)*,
pré-revogação em `logs/18-acceptance-pre-revoke.json` *(evidência removida do repositório público)*,
pós-revogação em `logs/18-acceptance-final.json` *(evidência removida do repositório público)*.

| Passo | Resultado observado | Evidência |
|---|---|---|
| Abrir solução restrita | caminho completo, `Restricted mode`, trust desmarcado, árvore textual; 12 métodos executáveis recusados com `trust_required`; edição/save de `Order.cs`; nenhum `bin/obj` criado | `18-acceptance-summary`, `S18-screen-01-trust.png` |
| Confiar + SDKs | host `10.0.100`; projeto `8.0.408` resolvido por `global.json`; TFM net8.0; `status-sdk` separa os dois | `S18-toolchain-trusted.png`, `L18-environment` |
| Árvore avaliada | console/biblioteca/teste, `TargetPath` Debug/net8.0, `bin/obj` ocultos; detalhes do projeto | `S18-screen-03-solution-explorer.png` |
| Restore explícito | build bloqueado com “Dependencies are not restored”; restore real exit 0; três projetos `restored`; reload revision 2 | `L18-restore.json` *(evidência removida do repositório público)* |
| csharp-ls | servidor 0.28.0.0, capabilities completion/definition/rename(prepare)/code actions; completion real `Id`; definição `Order.cs:4:11`; rename 3 arquivos/6 edições com cancelamento inerte; 6 code actions reais, prévia cancelada | `L18-lsp-session.json` *(evidência removida do repositório público)*, `L18-code-actions.json` *(evidência removida do repositório público)*, E08–E10 |
| Diagnóstico real | `CS0246` LSP `current` na revisão; Problems separa `LSP · csharp-ls` e `Build · dotnet` | `S18-screen-04-problems.png` |
| Build falho → correção | CS0246, exit 1, problema navegável; Run/Debug bloqueados; rebuild exit 0 | `L18-build-failed.json` *(evidência removida do repositório público)*, `L18-build-fixed.json` *(evidência removida do repositório público)* |
| Run | `dotnet <TargetPath>` do build identificado; stdout `Order A-104`; exit 0 | `L18-run.json` *(evidência removida do repositório público)*, `S18-screen-02-editor-build.png` |
| VSTest | descoberta 4; selected 1 passed; all 2 passed/1 failed/1 skipped exit 1; teste longo cancelado com 0 results parciais, TRX missing, exit 143 | `L18-tests-run-all.json` *(evidência removida do repositório público)*, `L18-tests-cancel.json` *(evidência removida do repositório público)*, `S18-screen-07-tests.png` |
| PTY | `/bin/zsh`, cwd real com espaço, `pwd`, `echo`, `sleep 30` + Ctrl+C, `dotnet --version` 8.0.408; close com decisão; shell exit `null` e `SIGTERM then SIGKILL`, `killedAfterGrace:true` | `L18-terminal.json` *(evidência removida do repositório público)*, `S18-terminal-started.png`, `S18-screen-06-terminal.png` |
| DAP experimental | opt-in, netcoredbg real `--interpreter=vscode`; breakpoint verificado, pausa, F8, locals, continue exit 0; segunda sessão stop/cancelled; processos debug `exited` | `L18-dap.json` *(evidência removida do repositório público)*, `S18-debug-paused.png`, `S18-screen-05-debug.png`, `S18-dap-stopped.png` |
| Revogação | buffer não salvo preservado, chamada direta `trust_required`, LSP parado, nenhum processo do workspace `running`; depois descarte explícito | `18-acceptance-summary` (`revoke`), `18-acceptance-final` |

Sete telas a 1440×900: `S18-screen-01-trust.png`, `S18-screen-02-editor-build.png`,
`S18-screen-03-solution-explorer.png`, `S18-screen-04-problems.png`, `S18-screen-05-debug.png`,
`S18-screen-06-terminal.png`, `S18-screen-07-tests.png`. Extras: `S18-build-failed.png`,
`S18-terminal-started.png`, `S18-debug-paused.png`, `S18-dap-stopped.png`,
`S18-tests-cancel.png`, `S18-layout-1100x700.png`.

## Auditoria de UI e acessibilidade

Resultado em `logs/18-ui-audit.json` *(evidência removida do repositório público)*. Verificações reais via Playwright
(Chrome 155), não certificação WCAG:

- `lang="en"`; amostra automatizada de seis termos PT (`Confiança`, `Salvar`, `Arquivo`,
  `Depurar`, `Cancelado`, `Falha`) não encontrada e inspeção manual das telas em inglês; nenhuma
  requisição externa observada (somente `localhost:1420`/`127.0.0.1:4317`).
- Nomes acessíveis: nenhum controle visível sem nome pela heurística no setup nem no workspace
  (`aria-label`, `aria-labelledby`, `label`, texto, `title`, placeholder).
- Foco: `outline: solid 2px` cobre na cor `rgb(240, 160, 121)` ao navegar por Tab.
- Contraste calculado dos tokens: fg/bg 16.4; muted/panel 9.01; accent/panel 8.06; error/bg 10.31;
  warning/bg 12.53; success/bg 12.34; borda de botão/raised 3.03. Não é medição de tela real.
- Layout: sem overflow horizontal em 1440×900 e 1100×700; Run e aba Tests dentro da viewport;
  árvore/painel inferior rolam localmente; status e painel dentro da janela em 1100×700.
  A verificação cobre os controles citados, não todos os pixels de cada painel.
- Tema escuro/cobre da referência: `--bg #101216`, `--panel #191d23`, `--fg #edf0f4`,
  `--accent #f0a079`; sans/mono; ícones/estados têm texto.
- Teclado macOS exercitado de verdade: Run `Ctrl+R` (suíte `run.spec`), Debug `Ctrl+D`
  (jornada), breakpoint `Cmd+F8` (jornada), step over `F8` (jornada); step into `F7` está em
  E14/código. Windows `Shift+F10` Run, `Shift+F9` Debug, `Ctrl+F8` breakpoint, `F8`/`F7` steps
  foram inspecionados no código (`src/WorkspaceView.tsx`, `src/BuildPanel.tsx`,
  `src/DebugPanel.tsx`), não executados em Windows.

Correções encontradas na auditoria (sem mudar contrato de host):

1. **Run macOS**: o handler usava `Cmd+R`, mas a SPEC, o rótulo `Run (Ctrl+R)` e o keymap
   IntelliJ/Rider exigem `Ctrl+R`. Corrigido em `src/WorkspaceView.tsx`; `tests/e2e/run.spec.ts`
   atualizado para pressionar `Control+R`.
2. **Layout 1100×700**: a faixa de avisos ficava com ~20 px e escondia trust/restore; o painel
   inferior podia esconder ações. A saída agora pode encolher (mín. 120 px), a faixa mantém
   mín. 90 px e o painel inferior reseta a rolagem ao trocar de aba (`src/styles.css`,
   `src/WorkspaceView.tsx`). Evidência `S18-layout-1100x700.png`.
3. **Terminal**: `.terminal-output-panel .panel-body` era `overflow:hidden` e podia esconder
   shell/cwd/controles; agora rola. `S18-terminal-started.png` mostra shell/cwd/controles.
4. **LSP**: “Partial coverage · partial coverage” duplicado foi removido; processo sem exit
   mostra “exit not observed” em vez de “exit unknown” (`src/ToolchainPanels.tsx`).
5. **Painel inferior**: scroll não vaza entre abas; o conteúdo de Tests/Debug/Terminal começa
   no topo ao trocar de aba.

Limitações manuais registradas separadamente: sem leitor de tela; a janela Tauri nativa não foi
dirigida/capturada (TCC no macOS); teclado Windows só foi inspecionado no código; contraste é
cálculo dos tokens, não medição de painel. Nada disso foi declarado “passou” além do que é
heurístico.

## Matriz de Debug (decisão registrada)

| Plataforma | Asset/flag | Prova real | Decisão |
|---|---|---|---|
| macOS ARM64 | `osx-arm64` 3.2.0-1092, community supported; flag experimental OFF por padrão, opt-in de sessão | E14 (launch, breakpoint, step, stop, async/multi-thread) + A18 (breakpoint verificado, F8, locals, continue exit 0, stop observado) | **Experimental; flag mantida.** Não é suporte de release. #256 segue risco não reproduzido. Nenhuma retirada de flag. |
| macOS Intel | Sem `osx-x64` na release 3.2.0-1092 | Não executado | **Indisponível/não verificado**; nenhuma versão substituta. |
| Windows x64 | `win64.zip` no metadata; não instalado/executado | Não executado | **Gate aberto**: exige smoke DAP próprio antes de qualquer aprovação Windows. |

Linux/outras arquiteturas Windows: fora do P0, sem promessa. O spike CLI ARM64 não foi usado como
prova DAP; a prova é o netcoredbg real em `--interpreter=vscode`.

## .NET: alegações permitidas

- Comprovado nesta máquina: fixture net8.0; SDK do projeto 8.0.408 fixado; host de análise
  10.0.100; hostroot com runtime 10 e SDK 8; build/run/testes/DAP no TFM net8.0.
- **net6**: best effort, não verificado; sem equivalência de garantia.
- **Outros TFMs .NET 8+**: não verificados; não herdam a aprovação de net8.
- **Windows/Intel .NET**: não executados.
- **Mínimos de SO**: não declarados; registradas apenas as versões reais (macOS 27.2/26B118).
- **Linux/outras arquiteturas**: fora do escopo; sem promessa.

## Auditoria de componentes, licenças e distribuição

Fontes reais: `cargo metadata --format-version 1` (workspace e `src-tauri`), `cargo tree
--target aarch64-apple-darwin`, `bun.lock`, `node_modules/*/package.json`, `tauri.conf.json`,
`src-tauri/icons/icon.png`, `dist/`. Saídas: `L18-licenses-summary.txt` *(evidência removida do repositório público)*,
`L18-licenses-app-graph.tsv` *(evidência removida do repositório público)*,
`L18-licenses-npm.tsv` *(evidência removida do repositório público)*. **Isto é auditoria técnica, não parecer jurídico.**

### Componentes diretos / efetivamente presentes

| Componente | Versão | Licença | Origem / distribuição | Situação |
|---|---|---|---|---|
| OpenRidr (host, host-http, shell) | 0.1.0 | Apache-2.0 declarada | código próprio | **Falta arquivo `LICENSE`/`NOTICE` na raiz**; obrigação Apache não cumprida no pacote |
| OpenIdea | revisão citada na SPEC | Apache-2.0 | não incorporado; sem checkout/fork local | **Nada reutilizado**; obrigação de código não incide; proveniência só documental |
| csharp-ls | 0.28.0 | MIT | NuGet.org, `dotnet tool install --tool-path`, baixado pelo app após consentimento | não incorporado ao código; aviso MIT deve acompanhar a ferramenta; assinatura NuGet não verificada (limite registrado) |
| netcoredbg | 3.2.0-1092 | MIT | asset oficial `Samsung/netcoredbg`, baixado pelo app após opt-in; osx-arm64 community supported | não incorporado ao código; SHA-256 do digest do GitHub; sem assinatura/notarização; aviso MIT deve acompanhar o binário |
| React / React DOM | 18.3.1 | MIT | npm, bundle do frontend | aviso MIT não empacotado |
| CodeMirror + `@codemirror/*` | 6.x (6.0.2, commands 6.11.1, language 6.13.1, legacy-modes 6.5.5, state 6.7.6, view 6.43.15) | MIT | npm, bundle | aviso MIT não empacotado |
| `@tauri-apps/api` | 2.12.4 | Apache-2.0 OR MIT | npm, bundle | aviso não empacotado |
| Tauri / `tauri-build` | 2.12.5 / 2.7.4 | Apache-2.0 OR MIT | crates.io, binário Rust | aviso não empacotado |
| portable-pty | 0.9.0 | MIT | crates.io, binário Rust | aviso MIT não empacotado |
| serde / serde_json / sha2 / ureq / zip / libc | 1.0.229 / 1.0.154 / 0.10.9 / 2.12.1 / 2.6.1 / 0.2.191 | MIT OR Apache-2.0 (zip/portable-pty MIT) | crates.io, binário Rust | avisos não empacotados |
| rustls / ring (via ureq) | 0.23.46 / 0.17.14 | Apache-2.0 OR ISC OR MIT / Apache-2.0 AND ISC | crates.io, binário | avisos não empacotados |
| Grafo Rust macOS ARM64 | 232 nomes de crates (233 pares nome+licença por versões duplicadas) | MIT/Apache/BSD/ISC/Zlib/Unicode/Unlicense/CDLA/0BSD/CC0; **5 MPL-2.0**; nenhum GPL/AGPL | crates.io | MPL exige aviso/source offer; nenhum `THIRD_PARTY_NOTICES` |
| MPL-2.0 | cssparser 0.37.0, cssparser-macros 0.7.1, dtoa-short 0.3.5, option-ext 0.2.0, selectors 0.38.0 | MPL-2.0 | via `tauri-utils`/`dom_query`; no grafo do binário | aviso e disponibilização do source das partes MPL pendentes |
| npm dev/build | Vite 6.4.5 MIT; TypeScript 5.9.3 Apache-2.0; Playwright 1.64.1 Apache-2.0; Tauri CLI 2.12.4 Apache-2.0 OR MIT; @vitejs/plugin-react 4.7.0 MIT; @types MIT; 96 instalados | maioria MIT/ISC/Apache; `caniuse-lite` CC-BY-4.0 (build-time) | npm | não distribuídos no bundle do app, exceto o que o bundle contém; `caniuse-lite` não aparece no `dist/` |
| Ícone `src-tauri/icons/icon.png` | 512×512, sha256 `0f22b5fe…4fa0143` | não documentada | asset default do Tauri (anel), não uma marca OpenRidr | **Identidade própria não preservada**; substituir por asset OpenRidr licenciado |
| Fontes | — | — | `system-ui`/`ui-monospace` | nenhuma fonte empacotada; sem obrigação de fonte |
| `dist/` | bundle minificado | — | build Vite | **sem `LICENSE`/`NOTICE`/`.map`**; avisos MIT/Apache/MPL não acompanham o bundle |
| `src-tauri/tauri.conf.json` | — | — | `bundle.active: false`, sem lista de ícones | **sem instalador/assinatura/notarização**; release de distribuição não configurado |

Observações de obrigação (não parecer jurídico):

- MIT/BSD/ISC/Unicode/Apache: preservar copyright e texto de licença na distribuição; Apache
  também exige `NOTICE` quando aplicável. Hoje não há arquivo de licença/avisos no repositório
  nem no bundle.
- MPL-2.0: avisar que os arquivos são MPL e disponibilizar o source correspondente; não foi
  identificada modificação local desses crates.
- `caniuse-lite` CC-BY-4.0: atribuição se for distribuído; o grafo indica uso de build, não
  presença no `dist/`.
- csharp-ls/netcoredbg: MIT; o app baixa os binários, não incorpora source. O aviso deve ser
  acessível ao usuário e acompanhar eventual redistribuição do binário baixado.
- OpenIdea: Apache-2.0 não incorporado; nenhuma licença de terceiro herdada. A inspiração está
  documentada, mas não há código para atribuir.
- Sem vsdbg, C# Dev Kit, Rider/ReSharper ou marca JetBrains no código/asset; “Rider” aparece
  apenas como nome do keymap compatível, como a SPEC permite. O ícone default do Tauri é a
  exceção de identidade a resolver.

## Decisão de release e checklist do ticket

Decisão completa: [release-decision.md](release-decision.md). Resumo: **BLOCKED / NOT APPROVED**.
Aprovado somente o núcleo macOS ARM64 executado + caminho DAP experimental; bloqueados Windows
(core/UI/DAP), macOS Intel, licenças/identidade, packaging/assinatura e smoke da janela nativa.
Mac sozinho não é entrega reduzida.

Checklist do ticket 18 (marcar só o que foi realmente provado):

- [x] **1. Jornada UI → host → ferramentas reais.** Executada no Chrome real + Vite + host-http
  (mesmo `dispatch`), com csharp-ls, dotnet 8.0.408, netcoredbg e PTY reais. A janela Tauri nativa
  não foi dirigida (TCC); isso está registrado como limitação, não como prova nativa.
- [x] **2. Matriz AC-01…AC-34 + cláusulas compartilhadas por ambiente.** `docs/acceptance-matrix.md`,
  com evidência/limitação por macOS ARM64, macOS Intel e Windows x64; sem contagens inventadas.
- [ ] **3. Validar núcleo P0 em Mac e Windows juntos.** Mac ARM64 verde; Windows não executado.
  Gate `G-WIN-CORE`/`G-WIN-DAP` aberto; CI local não existe e CI só CLI não prova UI. Mac sozinho
  não fecha este item.
- [x] **4. Auditar sete telas/tema/strings/foco/nomes/teclado/estados/rolagem.** Feito com
  Playwright/Chrome em 1440×900 e 1100×700, screenshots e `18-ui-audit.json`; correções aplicadas.
  Limitações: sem leitor de tela, janela nativa não dirigida, teclado Windows só por código.
- [x] **5. Matriz Debug separada.** Flag ARM64 OFF por padrão e mantida; DAP real em E14/A18;
  Intel sem asset e sem substituição; Windows exige smoke próprio e não foi executado.
- [x] **6. Declarar .NET 8+ só nos alvos comprovados.** net8.0/SDK 8.0.408/host 10.0.100; net6
  best effort não verificado; sem alegação de todos os TFMs; versões reais de SO; sem Linux.
- [ ] **7. Auditar licenças/avisos/identidade e preservar obrigações.** A auditoria foi feita
  (tabela acima + logs), mas faltam `LICENSE`/`NOTICE`/`THIRD_PARTY_NOTICES`, o ícone é o default
  do Tauri e não há revisão jurídica. Itens de preservação/identidade permanecem abertos.
- [x] **8. Produzir decisão rastreável de release.** `docs/release-decision.md`: approved/blocked/
  not_verified, gates obrigatórios e condições para publicar Mac+Windows juntos; release NÃO
  aprovado.

## Limitações e pendências honestas

- **Windows x64**: nenhum núcleo/UI/DAP/PTY/VSTest executado; sem host/CI Windows. Não há
  aprovação Windows nesta entrega.
- **macOS Intel**: nenhum núcleo executado; Debug indisponível por ausência de `osx-x64` 3.2;
  nenhuma versão substituta.
- **Janela Tauri nativa**: não dirigida/capturada (TCC); Chrome real é o mesmo build/UI e o mesmo
  `dispatch`, mas não é a janela nativa. Smoke humano continua recomendado.
- **Leitor de tela/WCAG**: auditoria heurística de nomes/foco/contraste/overflow; não é WCAG formal.
- **Analyzers/pull diagnostics**: `analyzerCoverage` permanece `not_verified`; só push diagnostics
  foi consumido. MTP é explicitamente não suportado; TRX só fornece resultados.
- **Integridade**: NuGet imprime “Skipping NuGet package signature verification”; netcoredbg usa
  SHA-256 do digest do GitHub, sem assinatura/notarização. O app não inventa hash/assinatura.
- **#256**: risco upstream não reproduzido; não é falha universal nem compatibilidade provada.
- **Ordem dos testes**: `zz-acceptance.spec.ts` roda por último para não alterar contadores/
  sessões assumidos pelos tickets; se a ordem mudar, revisar as suítes por ticket.
- **Licenças**: auditoria técnica concluída; obrigações de distribuição e revisão jurídica
  permanecem abertas. Nada aqui é parecer jurídico.

## Artefatos de evidência

- Matriz: [`docs/acceptance-matrix.md`](acceptance-matrix.md)
- Decisão: [`docs/release-decision.md`](release-decision.md)
- Spec de aceitação: [`tests/e2e/zz-acceptance.spec.ts`](../tests/e2e/zz-acceptance.spec.ts)
- Logs: `logs/18-acceptance-summary.json` *(evidência removida do repositório público)*,
  `logs/18-acceptance-pre-revoke.json` *(evidência removida do repositório público)*,
  `logs/18-acceptance-final.json` *(evidência removida do repositório público)*,
  `logs/18-restore.json` *(evidência removida do repositório público)*, `logs/18-build-failed.json` *(evidência removida do repositório público)*,
  `logs/18-build-fixed.json` *(evidência removida do repositório público)*, `logs/18-run.json` *(evidência removida do repositório público)*,
  `logs/18-tests-run-all.json` *(evidência removida do repositório público)*, `logs/18-tests-cancel.json` *(evidência removida do repositório público)*,
  `logs/18-terminal.json` *(evidência removida do repositório público)*, `logs/18-dap.json` *(evidência removida do repositório público)*,
  `logs/18-debug-install.json` *(evidência removida do repositório público)*, `logs/18-lsp-session.json` *(evidência removida do repositório público)*,
  `logs/18-ui-audit.json` *(evidência removida do repositório público)*, `logs/18-playwright-acceptance.log` *(evidência removida do repositório público)*,
  `logs/18-playwright-full.log` *(evidência removida do repositório público)*, `logs/18-licenses-summary.txt` *(evidência removida do repositório público)*.
- Screenshots: `screenshots/18-screen-01…07-*.png` + extras listados acima.
