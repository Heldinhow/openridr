# Evidência — Solution Explorer visual (2026-11-04)

Alvo: macOS arm64, bundle **debug** `0.3.0-alpha.6` construído deste worktree no HEAD `f6716c9`
(antes dos commits seguintes, que são só de E2E). Instância única,
`OPENRIDR_HOME` isolado em `<tmp>/verify-openridr-501-desktop/home`, fixture copiada para
`<tmp>/verify-openridr-501-desktop/work/OrderDesk` pelo `desktop.sh up`. O app foi fechado com
`desktop.sh down`; `pgrep -x openridr` e `launchctl getenv OPENRIDR_HOME` ficaram vazios.

## Comandos

```sh
bun run tauri build --debug --bundles app
sh .agents/skills/verify-openridr/scripts/desktop.sh up src-tauri/target/debug/bundle/macos/OpenRidr.app
agent-desktop snapshot --app OpenRidr -i --compact
agent-desktop click @<ref>
agent-desktop screenshot --app OpenRidr docs/evidence/solution-explorer-visual/<arquivo>.png
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

E2E (portas próprias, sem colidir com a paridade em 1430/4327):

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"
export DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en
export OPENRIDR_E2E_UI_PORT=1440 OPENRIDR_E2E_HOST_PORT=4337
export OPENRIDR_E2E_HOME=/tmp/openridr-e2e-home-solution-visual
export OPENRIDR_SCREENSHOT_DIR=/tmp/openridr-e2e-shots-solution-visual
bunx playwright test tests/e2e/solution-explorer.spec.ts tests/e2e/evaluation.spec.ts \
  tests/e2e/workspace.spec.ts tests/e2e/ui-layout.spec.ts tests/e2e/zz-acceptance.spec.ts --reporter=list
```

Resultado antes do merge: **43 passed, 0 failed, exit 0** (2,1 min). `zz-acceptance` sozinho: 2/2.
Specs: solution-explorer 11, evaluation 7, workspace 16, ui-layout 7, zz-acceptance 2.

Após o merge do `origin/main` (merge `19ccb6d`, PR #22), a mesma suíte foi repetida com
`focus-after-navigation.spec.ts`: **49 passed, 0 failed, exit 0** (2,3 min), incluindo os 7 testes de
foco (`focusEditorWhenMounted` preservado). `cargo test -p openridr-host`: 79 passed e a falha conhecida
`file_access_is_confined_to_the_workspace` (baseline do main). `bun run check`/`build` OK; `bun test` 21/21.

## O que foi verificado no app real

1. **Sem chrome de avaliação na árvore.** O snapshot de acessibilidade da árvore lista só nós
   (`OrderDesk · 3 projects`, `src`, projetos, `Dependencies`, arquivos). `MSBuild evaluated`,
   `Evaluate`, `revision N` e `Project details` não existem na árvore. O estado fica na status bar:
   snapshot completo traz `statictext 'Evaluated · Debug · TFM not selected · revision 2'` e o botão
   `Re-evaluate project model` (ícone de refresh). Captura: `01-tree-evaluated.png`.
2. **Projetos por tipo e cor.** Raiz de solução (blocos azuis), solution folders (pasta azul-cinza com
   contagem), `OrderDesk.Cli` (terminal azul), `OrderDesk.Core` (livro verde), `OrderDesk.Tests`
   (frasco âmbar). Capturas: `01-tree-evaluated.png`, `03-dependencies-tests.png`.
3. **Dependencies agrupadas.** `Frameworks` (camadas), `Packages` (caixa dourada, nome + versão) e
   `Projects` (pasta, ref com o ícone do projeto referenciado). Em `OrderDesk.Cli` e `OrderDesk.Core`
   `Packages` não aparece (grupo vazio omitido). Capturas: `02-dependencies-groups.png`,
   `03-dependencies-tests.png` (`Microsoft.NET.Test.Sdk 17.11.1`, `xunit 2.9.2`, `OrderDesk.Core`).
4. **Símbolo C# por arquivo.** Na cópia isolada foram criados `IWidget.cs` (interface), `Kind.cs`
   (enum), `Point.cs` (record) e `Money.cs` (struct), e o modelo foi reavaliado pelo botão da status
   bar (revision 2 → 4). A árvore mostrou os badges **I** (teal), **E** (âmbar), **R** (violeta),
   **S** (vermelho) e **C** (azul) para `Order.cs`; `Program.cs` (top-level statements) manteve o glifo
   C#. Captura: `04-symbol-badges.png`.
5. **Gerado/excluído esmaecido.** `Show All Files` listou `bin`/`obj` com `· no index`, fundo âmbar e
   texto/ícone atenuados; arquivos de fonte continuam coloridos. Captura:
   `05-show-all-no-index.png`.

## Não verificado no app real

- Aviso `Partial workspace` e classe de erro da status bar: cobertos por
  `qa4-eval-failed.spec.ts` (E2E), não por captura no app.
- Projeto `web` (não há fixture web) e tema claro (o build não tem token set claro).
- Hover/teclado no app: a árvore web não expõe foco por Tab ao agent-desktop (limite conhecido);
  o E2E cobre teclado no navegador.
- `Project details` via ⋮ no app: o menu não é exposto como ref estável; o E2E cobre.

## Falhas de E2E encontradas e causa

- `zz-acceptance` hardcodeava `127.0.0.1:4317`, `1420` e `openridr-e2e-home`; com portas próprias
  falhava por conexão e acusava requests locais como externos. Corrigido para importar
  `frontend`/`host`/`e2eHome` de `tests/e2e/environment.ts`.
- `ui-layout` e `zz` clicavam na linha do projeto (que também colapsa) e depois esperavam o arquivo;
  agora reabrem o projeto se `aria-expanded` ficou `false`.
- `zz` escrevia `LongTests.cs` e clicava Discover sobre o modelo avaliado stale; o host recusava com
  `evaluation_stale`. O teste agora clica Re-evaluate e espera a avaliação antes de descobrir;
  `clickDiscover` também espera avaliação e discovery não-`discovering`.
- `ui-layout` 1440 media a status bar com o novo botão de refresh; a leitura visível foi encurtada e o
  gap da barra reduzido para caber sem elipse. Nenhuma falha permaneceu no run final.
