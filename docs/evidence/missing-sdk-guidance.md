# Evidência — requisito de SDK, download oficial e "Check again" (#289)

Issues: #289 (mãe), #302, #303, #304. Ambiente: macOS ARM64, worktree isolada, SDK 8.0.408 e 10.0.403
em `~/.dotnet`. Data: 2026-11-08.

## O que mudou

- **O requisito de SDK tem duas origens.** `crates/host/src/sdk.rs` ganhou `SdkRequirement`
  (`source` = `global_json` | `target_framework` | `none`, `version`, `line`, `rollForward`,
  `allowPrerelease`). O pin do `global.json` continua tendo precedência; sem pin (ou com `global.json`
  sem `sdk.version`) a linha vem do TargetFramework declarado: `netN.0` com N≥5 pede SDK N.x ou maior,
  o sufixo de plataforma é ignorado (`net10.0-windows10.0.19041.0` → 10), multi-TFM usa a maior linha, e
  `netstandard*`/`netcoreapp*`/`net4x`/TFM desconhecido não afirmam requisito.
- **`no_global_json` e `not_pinned` deixaram de existir.** Os dois estados viraram
  `resolved`/`sdk_missing` com `requirement.source = "target_framework"`; os gates de
  evaluation/restore/build/run/debug/test/analysis continuam exigindo `state == "resolved"`, mas agora
  existe `resolved` sem pin. `sdk_missing` é o único estado de falha e carrega `requirement`,
  `detected` e `download`.
- **Sem pin a seleção segue o muxer.** Maior SDK estável instalado que satisfaça a linha; prerelease só
  quando é o único que satisfaz, e a mensagem diz que é prerelease; sem requisito, o maior estável.
  Nenhuma raiz de detecção nova.
- **A URL é montada no host, sem rede** (`download_for`): pin com versão → patch exato em
  `builds.dotnet.microsoft.com` + página da linha como alternativa; só linha → `aka.ms` da linha +
  página da linha; sem requisito → página geral, sem versão no rótulo. SO/arquitetura vêm de
  `std::env::consts`, a mesma fonte de `app.platform`/`app.arch`; plataforma desconhecida cai na página
  geral em vez de inventar URL.
- **Um bloco, três superfícies.** `src/welcome/ProjectSdkDownload.tsx` renderiza requisito, versões
  detectadas, mensagem do host, ação de download (+ alternativa), "Check again" e "Open global.json";
  é usado pela nota do Solution Explorer, pelo card "Project SDK" do painel Toolchain e pelo popover
  da barra de status (a barra é uma linha e não comporta link; o popover do `status-mode` já existia e
  é onde o bloco cabe, com `status-sdk-*` como prefixo dos testids para não colidir com a nota).
- **"Check again" chama o `sdk.redetect` que já existia.** Quando o redetect leva `sdk_missing` →
  `resolved`, a UI dispara a avaliação uma vez — a comparação é feita antes/depois da chamada, nunca
  observando o snapshot, então um re-render não avalia de novo. O host marca `evaluation`, `build`,
  `tests`, `test_run` e agora também `run` como `stale` (`Run::mark_stale`, que relabela sem cancelar o
  processo que o usuário iniciou).

## Falha-antes medida

- **Host (#302).** Com `crates/host/src/` de `9b2beb6` e os testes de fronteira novos: **2 failed**
  (`left: "no_global_json"`, `right: "resolved"`; `requirement` = `null`). Com a mudança: 2 passed.
- **Host (#304).** Sem a chamada `inner.run.mark_stale(...)`: **1 failed** —
  `run is not stale: {… "stale":false … }`. Com ela: 1 passed.
- **UI (#304).** Sem o stamp de `autoEvaluation.current` em `runEvaluation`: o E2E mediu
  `Expected length: 1 / Received length: 2 / Received array: ["evaluation.run","evaluation.run"]`.
  Com ele: verde.
- **UI (#303).** Com `src/` e `crates/host/src/` de `08ea6db` e só o spec novo: **2 failed / 3 passed**
  (`TypeError: … reading 'label'`; a nota antiga sem "No SDK detected").

## Bateria (nesta worktree, branch de integração)

- `cargo fmt --all --check` → limpo.
- `cargo clippy --workspace --all-targets -- -D warnings` → limpo.
- `cargo test -p openridr-host` → **647 passed, 0 failed, 70 ignored** (486 lib + 114 `host.rs` + 47
  outros).
- `bun run build` (`tsc --noEmit` + `vite build`) → limpo; `bun test tests/unit` → **139 passed**.
- Playwright em modo web, Chromium, portas 2820/5817:
  - `sdk-prerequisites.spec.ts` + `evaluation.spec.ts` → **14 passed** (7 + 7), incluindo o cenário
    "presente" com `fixtures/Net10NoPin` (`net10.0` sem `global.json`), os cenários "ausente" e
    "incompatível" por snapshot sobreposto, e o clique em "Check again" com a transição
    `sdk_missing` → `resolved` avaliando uma vez.
  - `medium-editor-state.spec.ts` + `trust-sdk.spec.ts` → **4 failed, 6 passed**. As quatro falhas são
    **pré-existentes**: reproduzem igual em `origin/main` (`8ab8cc6`) na mesma máquina — `b15`
    (altura do editor 156 < 170) e três de `trust-sdk`, todas por `eval-configuration` não existir fora
    do menu "Workspace actions" (o `data-testid` está dentro do `ToolbarMenu` desde `f7d92a1`, que está
    na `main`). A asserção do link de download que o #303 mudou em `trust-sdk.spec.ts:208` **passa**: o
    teste segue até a linha 232 antes de falhar.

## Revisão de dois eixos

Revisão em dois subagentes paralelos (Pi `openai/gpt-6.1-terra`, thinking medium), fixed point
`git merge-base origin/main HEAD`. Achados e o que foi feito:

- **Standards 1 (médio):** o skip do cenário "presente" em `evaluation.spec.ts` escondia uma regressão
  que falhasse a resolução. Corrigido: o skip vem do inventário detectado e o teste afirma `resolved`.
- **Standards 2 (médio):** o teste do "Check again" não provava que o clique chegou ao host. Corrigido:
  `page.waitForRequest` espera o `sdk.redetect` em cada superfície antes das asserções.
- **Spec 1 (médio):** a barra de status não renderizava o bloco, só um resumo. Corrigido: o bloco é
  renderizado no popover do `status-mode`, com `status-sdk-download`, `status-sdk-check-again` e
  `status-sdk-open-global-json`, e o cenário "incompatível" passa a afirmá-lo.
- **Spec 3 (médio):** o card do Toolchain chamava `evaluation.run` direto, fora da coordenação de
  "uma avaliação por contexto". Corrigido por remoção: a tela de boas-vindas não tem árvore, e a
  avaliação do projeto agora resolvido é a primeira do workspace, que já é uma por contexto.
- **Spec 4 (médio):** a mensagem de `sdk_missing` com pin não distinguia "nenhum SDK instalado" de "os
  instalados não servem". Corrigido em `sdk::resolve`, com teste dos dois casos.
- **Spec 5 (médio):** `tfm_sdk_line` aceitava `net10.7` como linha 10. Corrigido: só a forma `netN.0`
  (com sufixo de plataforma opcional) afirma requisito, com testes negativos para `net10.7`,
  `net10.0.1` e `net8.1-windows`.
- **Spec 6 (baixo):** o cenário "presente" recusava um SDK mais novo que 10. Corrigido: skip e
  asserção passam por "major ≥ 10".
- **Spec 7 (baixo):** `requirement.source` de um projeto sem pin e sem TFM com requisito era `none`,
  onde a entrada do ADR dizia `target_framework`. Mantido `none` (é o que deixa a tela dizer "o projeto
  não declara requisito" em vez de inventar uma linha) e a entrada do ADR ganhou a cláusula que
  registra o valor.
- **Spec 2 (alto) — respondido, não corrigido.** O snapshot nomeia o SDK que a regra escolhe, mas o
  MSBuild roda pelo muxer da raiz, que resolve `global.json` (inclusive um acima do workspace) e
  prereleases por conta própria: os dois concordam enquanto o `global.json` mais próximo for o da raiz
  do workspace e não houver prerelease mais novo que o maior estável. Fechar isso exige perguntar ao
  muxer a versão que ele resolve no diretório do projeto (ou parar de invocar `dotnet msbuild`), o que
  é mudança de outro contrato — a decisão do #289 fixa justamente a regra do host. Fica registrado como
  `shortcut:` em `sdk::resolve_unpinned` e como frente própria.

## Não verificado

- **Máquina sem SDK nenhum.** O cenário "ausente" é snapshot sobreposto no Playwright e teste unitário
  com `installs` vazio; esta máquina tem 8.0.408 e 10.0.403. O host não tem como esconder um SDK
  instalado do inventário real.
- **Instaladores de Windows e macOS Intel.** Só o mapeamento `os`/`arch`/extensão é unitário; nenhum
  download foi aberto.
- **`tests` e `test_run` `stale` com registro real.** O `mark_stale` deles é anterior a esta frente
  (#302) e o teste de fronteira prova `build` e `run` com processos reais; exercitar descoberta e
  execução de testes exigiria um projeto restaurado com pacotes de adaptador (rede) e um VSTest real.
- **A transição pelo card do Toolchain** (tela de boas-vindas): só o clique "ainda não atende" foi
  exercitado nessa superfície; a transição com reavaliação foi provada na nota da árvore.
- **App Tauri real.** A mudança é regra de host + UI pura, verificada por Playwright em modo web.
- **Windows e macOS Intel**, como no resto do produto.
- **`trust-sdk.spec.ts` no verde**: o spec está vermelho na `main` por motivo próprio, então a
  asserção editada só está provada até a linha 232 (acima).
