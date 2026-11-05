# OpenRidr 0.3.0-alpha.8 (pré-release)

> **Em preparo (não publicado), branch `feat/bundled-analysis-runtime`:** nesta build o
> `csharp-ls` 0.28.0 e o runtime .NET 10.0.14 que ele executa vêm **dentro** do app/DMG. Setup e
> reabertura não baixam a análise nem exigem runtime .NET 10 instalado; o SDK do projeto continua
> sendo o que o `global.json` pede, detectado na máquina (o app nunca instala SDK). O DMG de teste
> mediu 61.774.547 bytes (58,91 MiB). O texto abaixo descreve a alpha.8 **publicada**, cujo setup
> ainda baixava o csharp-ls (~99 MB) e exigia o runtime externo.

Oitava alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.
Esta alfa põe o **setup de ferramentas antes do workspace**, refaz **Tests** e **NuGet**, e fecha
o menu de contexto, os ícones de arquivo, o confinamento de arquivos, o ícone novo e os seis achados
da aceitação do `main`. Nada do funcionamento da alpha.7 regrediu.

## Novidades

- **Setup de ferramentas obrigatório, antes do trust.** Ao abrir um projeto, `csharp-ls` 0.28.0 e o
  `netcoredbg` embutido são verificados/instalados com **progresso medido (bytes e total)**, erro
  claro e retry, e a instalação verificada é compartilhada entre projetos: a segunda abertura vai
  direto ao workspace. O setup não exige trust e não executa código do projeto; análise, restore,
  build, test e debug continuam exigindo. Sem o runtime .NET 10 o app explica o requisito e oferece
  o link, sem instalar SDK nem tocar em `global.json`. O drawer deixou de ser gate manual e virou
  diagnóstico: versão, SHA-256, caminho, log e **Reinstall**.
- **Tests.** Abas (Explorer, All tests from project, Continuous Testing, Coverage), contadores por
  estado, árvore projeto → namespace → classe → teste com estado agregado, filtro, Expand/Collapse
  all, **Export** do último run de verdade e detalhe da seleção à direita (status, duração, projeto,
  origem). Debug de teste, Continuous Testing e cobertura aparecem **desabilitados com motivo** em
  vez de simular capacidade que o host não tem.
- **NuGet.** Escopo **Project/Solution**, fonte configurada, prerelease, Installed/Discover pela API
  v3 da fonte escolhida, seções recolhíveis com badge, **Implicitly Installed** lido de
  `obj/project.assets.json` e update em lote com resultado por pacote e um restore ao fim. CPM
  continua recusado com motivo. As consultas passam por fonte HTTPS validada, sem redirect e com
  DNS resolvido antes da conexão.
- **Ícones de arquivo na Solution Explorer (#28).** Cada arquivo ganhou glifo por tipo (C#, solução,
  projeto, JSON, markdown, Dockerfile, `.gitignore`), vendorizado de `@pierre/trees` (Apache-2.0) e
  do `c-sharp` do Simple Icons (CC0), com tipo exposto de forma acessível e contraste ≥ 3:1.
  `THIRD-PARTY-NOTICES.md` passou a cobrir `@pierre/trees`, Simple Icons e o T3 Code.
- **Menu de contexto do app em toda linha (#26).** Solução, projeto, pasta, arquivo e dependência
  abrem o menu do OpenRidr; o menu do WebKit saiu. Fechar devolve o foco à linha (teclado incluído) e
  a exceção é onde o menu do sistema **é** a funcionalidade: campo de texto e terminal.
- **Confinamento de arquivos (#25).** A decisão de contenção passou a ser tomada pela **forma** do
  caminho antes de qualquer `stat`/`canonicalize`: alvo fora do workspace responde
  `outside_workspace` mesmo quando não existe (negação fail-closed, sem oráculo de existência),
  symlink pendente é recusado e prefixo absoluto equivalente é aceito, na mesma regra usada por
  workspace, LSP, outline e NuGet. As duas falhas "conhecidas" do `cargo test` ficaram verdes:
  `cargo test --workspace` está **393/393**.
- **Novo ícone do app (#29).** O `OR` com moldura azul-violeta-rosa substituiu os PNG/`.icns`/`.ico`
  gerados do glifo antigo, e o cabeçalho da welcome usa o mesmo ícone.
- **Correções da aceitação do `main` (#30).** O **Export** de Tests grava o run real (antes saía
  `run.revision 0` e contadores zerados); o progresso do setup mostra **bytes e total** sem somar o
  mesmo `.nupkg` três vezes (19 → 44 → 68 → 82 → 100 % de 21.643.814 bytes); o painel de Tests nomeia
  o **TFM** em que opera e a avaliação atual ganha da descoberta anterior; abrir o menu de uma
  dependência **seleciona o projeto dono**; a barra "N selected for update" é renderizada **uma vez**;
  a barra de status virou `contentinfo` nomeada e o detalhe de build/test é região nomeada com
  `aria-live`.
- **Tamanho.** O DMG vai a **10,20 MiB** (10.699.101 bytes) e o app instalado a 24,25 MiB — o
  acréscimo frente à alpha.7 é dos ícones novos e do código de Tests/NuGet; as fontes e o netcoredbg
  embutido seguem como estavam.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.8_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra normalmente. Gatekeeper aceita sem diálogo: assinatura Developer ID + notarização estão no app e no DMG.
3. É preciso o .NET SDK que o projeto pede (o app só detecta, nunca instala nem edita `global.json`) e o **runtime .NET 10** para o `csharp-ls`. Na primeira abertura o setup baixa o csharp-ls (~99 MB, com progresso e retry) antes de pedir trust.
4. Para virar o app padrão de `.sln`/`.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr → **Alterar Todos**.

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.
- **.NET SDK** do projeto (o app não instala). A análise usa o **runtime .NET 10** (o csharp-ls o exige); o app não exige SDK 10 de um projeto que pede 8.

## Limitações conhecidas

- **O tema claro não foi verificado.** Os tokens de ícone usam `light-dark()` com fallback escuro,
  mas o build só distribui o tema escuro (Settings: "Light theme is unavailable") e os hex claros dos
  glifos ficam em 3–4:1, abaixo dos 4,5:1 do texto.
- **Projeto web só é detectado pelo texto do `.csproj`.** O tipo `web` vem de
  `<Project Sdk="Microsoft.NET.Sdk.Web">`, não de propriedade MSBuild avaliada, e não há fixture web
  para provar a tela.
- **A suíte E2E completa não passa no `main`.** Continuam falhando `find-usages` ("lists the real
  references", intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts`, 2 de `ui-layout`,
  além de `ui-fold`/`csharp-ls`, que passam isolados e são instáveis sob carga. Há ainda dívida
  conhecida em `testsuite.spec.ts:51`, que espera o `data-testid` `tests-filter-count` (o filtro
  atual só filtra a árvore): o spec falha sem que a UI esteja errada.
- **Dock Quit e AppleScript `quit` não perguntam.** Esses caminhos usam `terminate:` nativo do macOS, que o Tauri não expõe; ⌘Q, o menu Quit e o botão Close perguntam.
- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto; o produto informa a lacuna e não habilita um botão falso.
- **Continuous Testing e cobertura não existem.** Aparecem desabilitados com motivo, como o debug de teste.
- **Atualizar o netcoredbg exige release do app.** A versão é fixada e assinada com o app; o Reinstall reinstala a mesma versão verificada.
- **O csharp-ls continua sendo baixado pelo usuário.** O setup obrigatório baixa o pacote pinado (0.28.0, SHA-256 conferido) e o compartilha entre projetos; embutir a análise exigiria carregar o runtime .NET junto, fora do escopo.
- **NuGet: update em lote e install descoberto foram provados numa cópia descartável da fixture.** A fixture versionada não foi alterada, e o escopo Solution no app real agrega só os projetos que ele conhece.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **`.slnx` abre, mas não restaura nem builda.** O SDK 8.0.408 pinado não suporta `.slnx`; a falha aparece como erro, sem sucesso inventado.
- **Abrir pelo Finder/Dock foi verificado por `open -a`.** Duplo clique com mouse e arrasto real para o Dock não foram exercitados.
- **`Escape` para fechar o menu de contexto foi provado no Chromium.** No app real os menus foram fechados por clique físico; o `AXCancel` do macOS não fecha menu React.
- **O glifo de documento (`.sln`/`.csproj`) tem prova unitária.** Essas linhas não aparecem no Show All Files, então não foram vistas na árvore do app.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.slnx`, `.csproj` ou pasta).
- Tags 13–16 da paridade ficaram **adiadas**: debug de teste, Add Existing Item, Move e Exclude continuam sem implementação.
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado.
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
bed67e4347e7ba40d4d5d396a4fb5345842ef0daf1baad9c8f814e2ed7ae4b0a  OpenRidr_0.3.0-alpha.8_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: 10,20 MiB (10.699.101 bytes). Verificado no DMG
montado: `CFBundleShortVersionString` 0.3.0-alpha.8, binário Mach-O arm64,
`codesign --verify --deep --strict` e `xcrun stapler validate` no app e no DMG,
`spctl --assess --type execute` no app = *accepted, source=Notarized Developer ID* e
`spctl --assess --type open` no DMG = *accepted, source=Notarized Developer ID*. O app foi aberto a
partir do DMG montado, com `OPENRIDR_HOME` isolado e a fixture `OrderDesk`: o setup de ferramentas
rodou antes do trust, a árvore abriu com os ícones novos, o painel de Tests mostrou
`Discovered 4 tests · exit 0` em `net8.0`, o NuGet mostrou escopo Project/Solution e
**Build succeeded · exit 0** (0 warnings, 0 erros).

Nesta release, no HEAD `0fbd844` + este commit: `bun run check` e `bun run build` OK, `bun test`
**33/33**, `cargo test --workspace` **393/393** (263 lib + 14 debugger + 98 host + 10 test_discovery
+ 7 test_run + 1 trust_model), sem nenhuma falha.

## Notas de licença

O app distribui `netcoredbg` 3.2.0-1092 (MIT, Copyright © 2017 Samsung Electronics Co., LTD),
componentes Roslyn/dbgshim (MIT), Lucide (ISC e MIT/Feather), Material Design Icons (Apache-2.0),
os glifos de árvore de `@pierre/trees` (Apache-2.0), o `c-sharp` do Simple Icons (CC0 1.0), a
referência visual do T3 Code (MIT) e as fontes Inter e JetBrains Mono (OFL-1.1). O texto completo
está em `Contents/Resources/THIRD-PARTY-NOTICES.md`.
