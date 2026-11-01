# RIDR-VS-RIDER — fluxo "abrir → inicializar → run → debug"

Data: 29/10/2026. Referência: JetBrains Rider (Trial, tema escuro) aberto em `/tmp/OrderDesk` (cópia de `fixtures/OrderDesk`). OpenRidr: app desktop via `bun run tauri dev` (janela Tauri real, SDK 8.0.408 em `~/.dotnet`) e, para o fluxo de primeira abertura, ponte web em Chrome visível com `slowMo` 300 ms e home limpo. Capturas lado a lado (Rider à esquerda, OpenRidr à direita) em `docs/evidence/rider-compare/fluxo-*.png`.

## Como foi feito (e limites)

- O Rider já estava aberto com outro projeto; abri `OrderDesk.sln` numa janela nova (`open -a Rider`), então **a tela inicial do Rider não foi exercitada** — só o equivalente de File > Open. Não apareceu diálogo de trust no Rider para essa pasta.
- No primeiro Debug o macOS abriu "Developer Tools Access" pedindo senha/Touch ID. **Cancelei, sem credenciais**; o Rider seguiu e parou no breakpoint mesmo assim.
- Cliques nos apps nativos por `CGEvent` (JXA) e capturas por `screencapture`. O Playwright MCP não tinha permissão nesta sessão; usei scripts Node com Playwright (`.scratch/compare/`, não versionado).
- O `/Applications/OpenRidr.app` instalado é um build antigo (sem as correções abaixo). Encerrei esse processo para rodar o `tauri dev`. **Falta gerar um `.app` novo** (`bun run tauri build`); não fiz.
- Step into/out e Continue não foram reexercitados nos dois apps (só Step Over); seguem cobertos por `tests/e2e/debugger.spec.ts`. Git e Problems não foram reexercitados: a cópia em `/tmp` não é repositório; vale o `docs/UI-AUDIT.md` (A3, M2).

## Relatório por passo

| # | Passo | Rider | OpenRidr (antes) | Divergência | Gravidade |
|---|---|---|---|---|---|
| 1 | Abrir projeto | File > Open (ou tela inicial) abre a solution; árvore e Run config aparecem já ([fluxo-01](evidence/rider-compare/fluxo-01-open-project.png)) | Tela inicial com recentes + **Open** (seletor nativo `osascript`) abre em Restricted e entra no IDE. Funciona. O seletor só aceitou teclado depois de clicar nele | Sem menu File/Open no IDE; na ponte web são 4 passos (colar caminho, checkbox, Trust, Open workspace) | baixa |
| 2 | Trust | Um diálogo "Trust and Open" | Banner "Trust workspace…" abria um popover **cortado em 44 px**: só o checkbox aparecia, o botão "Trust and inspect project" ficava escondido ([fluxo-02](evidence/rider-compare/fluxo-02-trust-before-fix-clipped.png)). Depois de confiar, o popover ficava aberto sobre o editor | `overflow-x:auto` na toolbar também cortava o eixo y; o mesmo valia para o menu da run config e o ⋮ | **bloqueante** |
| 3 | Restore / indexação | Indexa e restaura sozinho; Run funciona logo | **Histórico:** "Restore not requested" e TFM "not selected" até o usuário pedir restore. **Atualizado em 2026-11-01:** um workspace Trusted (SDK resolvido) restaura sozinho uma vez por contexto ao abrir e reavalia ao terminar; o host não restaura implicitamente por `restore.run`, em Run/Debug nem em Restricted, e falha de restore continua `failed` com "Retry restore" | Restore explícito fora do workspace Trusted; falha não repete sozinha | média |
| 4 | Run config | Dropdown com a config + "Edit Configurations…" | Dropdown cortado (mesma causa do passo 2), config derivada só leitura | Rodapé do diálogo é Save/Duplicate/Remove/Cancel, não Run/Apply/OK (UI-AUDIT M3, não reverificado) | **alta** (corte) / média (diálogo) |
| 5 | Run + saída | ▶ faz build e abre a janela Run com a saída em ~10 s ([fluxo-04](evidence/rider-compare/fluxo-04-run-output.png)) | **Histórico** (medição de 2026-10-29): Run ficava desabilitado ("Build the startup project first; Run never builds or restores implicitly") e exigia restore, TFM e Build manuais. **Atualizado em 2026-11-01:** Run e Debug iniciam o build implicitamente quando não há build atual e mostram `building`; restore continua explícito. Saída segue na aba Run/Build | A saída aparece na aba Run (e o log de build na aba Build), não numa janela Run separada | resolvido (Run) / média (janela separada) |
| 6 | Debug | 🐞 faz build, inicia e para no breakpoint ([fluxo-06](evidence/rider-compare/fluxo-06-debug-paused-after-fix.png)) | **Histórico:** clicar em Debug só abria a aba Debug e nada acontecia ([fluxo-05](evidence/rider-compare/fluxo-05-debug-click-before-fix.png)): o consentimento da flag experimental usava `window.confirm`, que o WKWebView do Tauri não exibe (devolve `false`). Depois disso o fluxo pediu um diálogo próprio ("Enable and debug") por algumas fatias. **Atualizado em 2026-11-01 (alfa 3):** um clique em Debug ativa a flag experimental sozinho, reusa/instala o netcoredbg aprovado só se faltar e inicia a sessão; nenhum modal. O host builda implicitamente quando não há build utilizável | O diálogo era o último atrito; removido | resolvido |
| 7 | Step over / frames / variáveis | Barra com Resume/Pause/Step Over/Into/Out, abas Threads & Variables / Console / Debug Output, "Evaluate expression", variável expansível ([fluxo-07](evidence/rider-compare/fluxo-07-step-over.png)) | Step over funciona, frame e locals corretos, inlay `args = …, order = …` na linha. Objeto aparece como `{OrderDesk.Core.Order} (nested; not expanded)` | Sem expandir variáveis, sem "Evaluate expression", ícones de passo sem rótulo, painel sem abas de Console/Debug Output | média |
| 8 | Terminal | Shell abre sozinho com prompt ([fluxo-09](evidence/rider-compare/fluxo-09-terminal.png)) | Mostra "Not started" + botão **Start shell**; entrada é uma linha separada com "Send line" | Não é um terminal interativo; buffer cru (UI-AUDIT M5) | média |
| 9 | Settings / Keymap | Dialog com árvore e busca ([fluxo-10](evidence/rider-compare/fluxo-10-settings.png)) | Dialog com busca e Appearance/Keymap/Editor/Build/Version Control; abre e fecha | Equivalente no essencial; keymap em lista de botões (UI-AUDIT M4) | baixa |
| 10 | Barra superior / layout | Header com projeto, branch, Build, config, ▶, 🐞, ⋮, busca, Settings; painel inferior fechado ao abrir | Mesma composição. Em janelas estreitas os itens passam a ser cortados (ver "O que resta") | Layout geral aproximado | baixa |

## O que foi corrigido

1. `2a107f0` **fix(run): one-click Run and in-app Debug consent**
   - Debug: `window.confirm` virou o diálogo `debug-consent-dialog` ("Enable and debug"). Provado no app Tauri: dialog → instala/reusa netcoredbg → restore → build → para no breakpoint da linha 3 com frames e locals; Step Over andou para a linha 4.
   - Run: com Run bloqueado e workspace Trusted, o clique restaura (se preciso), avalia, builda e roda (`prepareAndRun`, compartilha `prepareBuild` com Debug). Provado no app Tauri: `Order A-104: R$ 42,00 · exit 0`. O atalho/ação `run.start` continua estrito.
   - Ambos passam a respeitar a run config ativa (config, TFM, args, env). Decisão registrada em `docs/decisions.md`.
   - Specs: `debugger.spec.ts` (one-click Debug, ajustado ao diálogo) e novo `run.spec.ts` (one-click Run).
2. `b7f9cc0` **fix(ui): toolbar dropdowns no longer clipped and close on outside click**
   - `overflow-x:clip` na toolbar; menus `<details>` fecham em clique fora e quando o trust muda. Provado no app Tauri (popover de trust completo, dropdown da run config completo).
   - Spec novo em `workspace.spec.ts` falha sem o CSS e passa com ele.
3. Alfa 3 (2026-11-01) **Debug sem pergunta**
   - O diálogo `debug-consent-dialog` foi removido: atalho, toolbar e `debug-launch` usam `prepareAndDebug`, que liga a flag experimental (macOS arm64), reusa/instala o adaptador só se faltar e chama `debug.start`. O painel Debug passa a mostrar `dap-experimental-note` como aviso não bloqueante.
   - A escolha de trust deixou de depender do popover da barra de status para Build/Run/Debug: o próprio motivo bloqueado mostra "Trust this project" (`build-trust`, `run-trust`, `dap-trust`), com `trust.grant {remember: true}`.
   - Evidência: `docs/evidence/alpha3-debug-trust.md`.

Verificação: `bun run check` ok; Playwright verde em `shell`, `ui-layout`, `workspace`, `settings`, `trust-sdk`, `run-configs`, `run.spec.ts` (novo teste) e no teste one-click de `debugger.spec.ts`. A suíte inteira não foi rodada; `debugger.spec.ts` completo só rodou antes do ajuste do spec (o único vermelho era o `confirm` antigo).

## O que resta

- **alta** — ~~Restore/avaliação/TFM não rodam ao abrir o workspace; hoje só rodam dentro de Run/Debug em um clique.~~ **Resolvido em 2026-11-01:** um workspace Trusted restaura sozinho uma vez por contexto ao abrir e avalia ao terminar (e de novo quando um arquivo de projeto muda); o restore do host continua explícito (sem restore implícito em `restore.run`, Run/Debug ou Restricted) e falha exige "Retry restore".
- **alta** — Gerar e instalar um `.app` novo: o instalado em `/Applications` ainda tem os bloqueantes.
- **alta** — O primeiro Debug baixa o netcoredbg e exige a flag experimental (macOS arm64); com o adapter ausente e sem rede o Debug falha com mensagem, mas não há progresso visível além do texto "Preparing Debug…". O clique já não pergunta nada (alfa 3): a flag é ligada e a instalação confirmada pelo próprio fluxo.
- média — Janela Run própria (saída hoje na aba Build); variáveis expansíveis e "Evaluate expression"; Terminal que inicia sozinho e interativo; diálogo de run config com Run/Apply/OK; trust num único diálogo.
- média — Com `overflow-x:clip`, abaixo da largura necessária os itens da direita (Run/Debug) são cortados sem rolagem; os specs de 1100×700 passam, mas não medi larguras intermediárias com muitos widgets.
- baixa — Menu File/Open no IDE; sobreposição de texto no item da run config ("Run OrderDesk.CliDerived from…").
- Não verificado: Windows, macOS Intel, Git/Problems nesta rodada, Step Into/Out no app Tauri.
