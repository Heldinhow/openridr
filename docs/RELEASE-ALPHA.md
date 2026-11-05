# OpenRidr 0.3.0-alpha.7 (pré-release)

Sétima alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.
Esta alfa entrega o polimento visual (ícones e fontes), a paridade com o Rider dos tickets 01–12 e
a nova Solution Explorer. Nada do funcionamento da alpha.6 regrediu.

## Novidades

- **Polimento de ícones e fontes.** Os 38 ícones desenhados à mão viraram um conjunto único
  (Lucide), o selo "C#" virou glifo neutro e os selos de texto de 8–9 px saíram da árvore e das
  abas. **Inter Variable** e **JetBrains Mono Variable** (latin) passam a ir embutidas no app, com
  ligaduras desligadas, escala tipográfica em quatro tokens e alvos de 32×32 na toolbar da árvore:
  a UI deixou de depender da fonte instalada na máquina. O `THIRD-PARTY-NOTICES.md` agora cobre
  Lucide (ISC + MIT/Feather), MDI (Apache-2.0) e Inter/JetBrains Mono (OFL-1.1).
- **Paridade com o Rider (tickets 01–12).** Quando falta análise, Find Usages, Rename e code
  actions oferecem **instalar ou iniciar** ali mesmo, e a consulta pendente roda sozinha quando a
  análise fica pronta. **⌘T** e duplo Shift abrem Search Everywhere; **⌘1…⌘8** abrem, focam e fecham
  as janelas de ferramenta; **⇧F12** e **⌥F7** fazem Find Usages. A descoberta de testes é
  automática por projeto (com a causa visível quando falha) e o painel ganhou **Next failure** e
  **Run Unit Test**. Stack trace de teste e `arquivo(linha,coluna)` do Build output são links que
  abrem o arquivo na posição. Menus de solução, projeto, pasta e arquivo têm Copy Path, Find in
  Files, Show in Finder, Rename e Delete; "Add Project" aparece desabilitado com o motivo. O Find do
  editor mostra `n of m`, breakpoint em linha sem código vem marcado (vazado) em vez de sumir,
  configuração de execução derivada ganhou "Duplicate to edit" e a barra de status nomeia o keymap
  ativo. Depois de abrir um arquivo pela busca, pelo Find Usages ou pelos Problems, o **foco vai
  para o editor** (retry por frame, sem roubar foco de diálogo, terminal ou campo de texto).
- **Solution Explorer visual (tickets 01–04).** A árvore perdeu o chrome de avaliação
  (`MSBuild evaluated`, `Evaluate`, `revision N`, `Project details`): o estado agora vive só na barra
  de status, com botão de re-avaliar. Cada projeto mostra **ícone e cor por tipo** (console,
  biblioteca, testes, web) e cada `.cs` traz o **badge do símbolo** (C, I, E, R, S); `web` vem do
  texto do `.csproj` (`Microsoft.NET.Sdk.Web`), sem inventar propriedade. **Dependencies** vem
  agrupado em **Frameworks**, **Packages** (nome e versão) e **Projects**, e grupos vazios não
  aparecem. Com `Show All Files`, `bin`/`obj` aparecem esmaecidos e marcados `no index`.
- **Tamanho.** O DMG vai a **10,13 MiB** (10.620.421 bytes) e o app instalado a 24,02 MiB. O
  acréscimo frente à alpha.6 vem das fontes embutidas (≈132 kB de woff2) e dos ícones novos.
- **Fechamento da alpha.6.** Assinatura Developer ID + notarização do app e do DMG, netcoredbg
  3.2.0-1092 embutido, SDK/ferramentas por projeto, quitar com confirmação, preservar CRLF/UTF-16,
  árvore com criar/renomear/apagar, abrir pelo Finder/Dock/terminal e Build Solution continuam como
  estavam.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.7_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra normalmente. Gatekeeper aceita sem diálogo: assinatura Developer ID + notarização estão no app e no DMG.
3. É preciso o .NET SDK que o projeto pede (o app só detecta, nunca instala nem edita `global.json`). A análise C# baixa o csharp-ls (~99 MB) na primeira abertura de projeto, com progresso e retry.
4. Para virar o app padrão de `.sln`/`.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr → **Alterar Todos**.

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.
- **.NET SDK** do projeto (o app não instala). A análise usa o **runtime .NET 10** (o csharp-ls o exige); o app não exige SDK 10 de um projeto que pede 8.

## Limitações conhecidas

- **O tema claro não foi verificado.** Os tokens de ícone usam `light-dark()` com fallback escuro,
  mas o build só distribui o tema escuro (Settings: "Light theme is unavailable"); o claro nunca foi
  visto no app.
- **Projeto web só é detectado pelo texto do `.csproj`.** O tipo `web` vem de
  `<Project Sdk="Microsoft.NET.Sdk.Web">`, não de propriedade MSBuild avaliada, e não há fixture web
  para provar a tela.
- **Falhas de teste conhecidas no main, que continuam aqui.** E2E: `find-usages` ("lists the real
  references", intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts`, 2 de `ui-layout`
  (falham também no HEAD sem mudanças), além de `ui-fold`/`csharp-ls` que passam isolados e são
  instáveis sob carga. Rust: `file_access_is_confined_to_the_workspace` e, sem restore prévio,
  `evaluation_returns_the_real_msbuild_model_for_every_project`.
- **Dock Quit e AppleScript `quit` não perguntam.** Esses caminhos usam `terminate:` nativo do macOS, que o Tauri não expõe; ⌘Q, o menu Quit e o botão Close perguntam.
- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto; o produto informa a lacuna e não habilita um botão falso.
- **Atualizar o netcoredbg exige release do app.** A versão é fixada e assinada com o app; não há update independente da ferramenta.
- **O csharp-ls continua sendo baixado pelo usuário.** Embutir a análise exigiria carregar o runtime .NET junto, fora do escopo.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **`.slnx` abre, mas não restaura nem builda.** O SDK 8.0.408 pinado não suporta `.slnx`; a falha aparece como erro, sem sucesso inventado.
- **Abrir pelo Finder/Dock foi verificado por `open -a`.** Duplo clique com mouse e arrasto real para o Dock não foram exercitados.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.slnx`, `.csproj` ou pasta).
- Tags 13–16 da paridade ficaram **adiadas**: debug de teste, Add Existing Item, Move e Exclude continuam sem implementação.
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado.
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
2fc75e6c441a6ddf31c313429b76eeef831c443b231d6f4b63c32c7d3a5567bc  OpenRidr_0.3.0-alpha.7_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: 10,13 MiB (10.620.421 bytes). Verificado no DMG
montado: `CFBundleShortVersionString` 0.3.0-alpha.7, binário Mach-O arm64,
`codesign --verify --deep --strict` e `xcrun stapler validate` no app e no DMG,
`spctl --assess --type execute` no app = *accepted, source=Notarized Developer ID* e
`spctl --assess --type open` no DMG = *accepted, source=Notarized Developer ID*. O app foi aberto a
partir do DMG montado, com `OPENRIDR_HOME` isolado e a fixture `OrderDesk`: trust concedido, Solution
Explorer sem chrome de avaliação, Dependencies agrupadas e **Build succeeded · exit 0**.

Nesta release, no HEAD `dd84990` + este commit: `bun run check` e `bun run build` OK, `bun test`
21/21, `cargo test --workspace` 235 + 14 + 78 testes verdes com as 2 falhas conhecidas acima, e
E2E de `solution-explorer` + `focus-after-navigation` **17/17** em portas próprias.

## Notas de licença

O app distribui `netcoredbg` 3.2.0-1092 (MIT, Copyright © 2017 Samsung Electronics Co., LTD),
componentes Roslyn/dbgshim (MIT), Lucide (ISC e MIT/Feather), Material Design Icons (Apache-2.0) e as
fontes Inter e JetBrains Mono (OFL-1.1). O texto completo está em `Contents/Resources/THIRD-PARTY-NOTICES.md`.
