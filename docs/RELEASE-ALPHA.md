# OpenRidr 0.3.0-alpha.9 (pré-release)

Nona alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.
Esta alfa faz a maior mudança de arquitetura desde a primeira: o **analisador agora viaja dentro do
app** — `csharp-ls` 0.28.0 e o runtime `.NET 10.0.14` privado que ele executa estão em
`Contents/Resources` —, e o resto é polimento visual, correção de debug, ícones da árvore no estilo
Rider e Git com Commit/diff lado a lado. Nada do funcionamento da alpha.8 regrediu.

## Novidades

- **Análise embutida (#32).** `csharp-ls` 0.28.0 e o runtime `.NET 10.0.14` privado vêm **dentro** do
  `.app`/DMG, ao lado do `netcoredbg` que já vinha. Primeira abertura e reabertura **não baixam nada**
  e não exigem runtime .NET 10 instalado: `setup.start` resolve o par, confere `--version` uma vez e
  fica `ready` com `plan: null`. O SDK do projeto continua externo, o que o `global.json` pede,
  detectado na máquina (o app nunca instala SDK nem edita `global.json`). O `dotnet` embutido executa
  o analisador a partir de um *hostroot* montado por symlink no home do app, com o `hostfxr` 10.0.14
  do bundle e o worker no runtime do SDK do projeto. Se o par embutido estiver faltando ou danificado,
  o app **falha fechado** ("Reinstall OpenRidr") em vez de voltar a baixar. A integridade do par é a
  assinatura Developer ID + notarização do app: o host não compara hash fixado em runtime e o SHA-256
  mostrado na UI é informativo. Build de release usa `scripts/vendor-analysis.sh`, com os artefatos
  pinados por SHA-512 e cada Mach-O assinado com hardened runtime (`allow-jit` +
  `disable-library-validation` no muxer).
- **Polimento visual estilo Rider (#33).** Ilhas, tokens escuros e cabeçalho Solution compacto;
  seleção única na árvore e abas com ícones coloridos; status compacto; SDK/runtime no popover de
  confiança. Sem dependência nova.
- **Debug: SDK do projeto no PATH do adapter (#34).** O `netcoredbg` passou a ser iniciado com o
  `<project SDK root>/dotnet` exigido e com o SDK do projeto **à frente** do PATH, e o `dotnet` do SDK
  vai no ambiente do launch DAP. Isso corrige o `configurationDone` falhando com `0x80070002` quando o
  app é aberto pelo Finder (PATH sem o SDK) — o runtime privado de análise não substitui o SDK do
  projeto; se o executável do SDK estiver ausente, o erro informa o caminho.
- **Ícones da árvore no estilo Rider (#35).** `.cs` com glifo `C#` verde (sem badge; o tipo de símbolo
  continua acessível pela descrição da linha), projeto em caixa `C#` verde, pastas em contorno,
  `Dependencies` em nós, framework em engrenagem, `.DotSettings` como XML e `bin`/`obj`/`.idea`/
  `.gitignore`/`global.json` com fundo discreto "no index". SVGs próprios/Lucide — nenhum asset da
  JetBrains.
- **Git: Commit e diff lado a lado (#36).** No painel Commit, grupo "Changes N files" tri-state e
  recolhível, toolbar, ícones e cores por status, nome + diretório na mesma linha e mensagem maior.
  O diff abre como aba do editor ("Commit: \<arquivo\>"), com dois CodeMirror read-only lado a lado,
  numeração, faixas por tipo, destaque intra-linha, próxima/anterior diferença, "N/M files" com ← → e
  contador de diferenças. Sem operação nova no host e sem dependência nova; rollback e Commit and
  Push… aparecem desabilitados com motivo.
- **Tamanho.** O DMG vai a **66,34 MiB** (69.559.207 bytes) e o app instalado a 163,6 MiB
  (171.549.682 bytes). O salto frente à alpha.8 (10,20 MiB) é o runtime .NET 10.0.14 mais o
  `csharp-ls` embutidos — o download único do DMG substitui os ~99 MB que o setup baixava na primeira
  abertura.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.9_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra normalmente. O macOS mostra **uma** confirmação de primeira abertura por ser um app baixado
   da internet ("Tem certeza de que deseja abri-lo?"); a assinatura Developer ID + notarização estão
   no app e no DMG, então não há bloqueio nem aviso de desenvolvedor não identificado. Confirmando
   uma vez, o app abre direto.
3. Não é preciso instalar runtime .NET 10 nem esperar download de ferramentas: a análise vem no app.
   É preciso o **.NET SDK que o projeto pede** (o app só detecta, nunca instala nem edita
   `global.json`).
4. Para virar o app padrão de `.sln`/`.csproj`: Finder → Obter Informações → **Abrir com** →
   OpenRidr → **Alterar Todos**.

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.
- **.NET SDK** do projeto (o app não instala). O runtime .NET 10 de que o `csharp-ls` precisa já está
  embutido e não é procurado na máquina; o app também não exige SDK 10 de um projeto que pede 8.

## Limitações conhecidas

- **A confirmação de primeira abertura do macOS é um passo do usuário.** Como todo app baixado, a
  primeira execução pede um "Abrir" no diálogo do sistema; depois disso o app abre sem perguntar.
- **O tema claro não foi verificado.** Os tokens de ícone usam `light-dark()` com fallback escuro,
  mas o build só distribui o tema escuro (Settings: "Light theme is unavailable") e os hex claros dos
  glifos ficam em 3–4:1, abaixo dos 4,5:1 do texto.
- **Projeto web só é detectado pelo texto do `.csproj`.** O tipo `web` vem de
  `<Project Sdk="Microsoft.NET.Sdk.Web">`, não de propriedade MSBuild avaliada, e não há fixture web
  para provar a tela.
- **A suíte E2E completa não passa no `main`.** Continuam falhando `find-usages` ("lists the real
  references", intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts` (`tests.spec.ts:317`),
  2 de `ui-layout`, além de `ui-fold`/`csharp-ls`, que passam isolados e são instáveis sob carga. Há
  ainda dívida conhecida em `testsuite.spec.ts:51`, que espera o `data-testid`
  `tests-filter-count` (o filtro atual só filtra a árvore). Nesta release os E2E **não** foram
  executados: a verificação de UI foi feita no app desktop real.
- **`csharp-ls.spec.ts:150` (Problems depois de SIGSTOP/SIGCONT) segue aberto** desde o #32, sem
  causa demonstrada: o servidor vira zumbi na sequência em `eb3fe6f` e no HEAD, então não há
  evidência de regressão — mas também não há prova de que a republicação sobreviva a esse sinal.
- **Atualizar o analisador ou o netcoredbg exige release do app.** As versões são fixadas, assinadas e
  notarizadas com o app; não há "Reinstall" que troque de versão, e o par embutido é a referência de
  integridade.
- **Dock Quit e AppleScript `quit` não perguntam.** Esses caminhos usam `terminate:` nativo do macOS, que o Tauri não expõe; ⌘Q, o menu Quit e o botão Close perguntam.
- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto; o produto informa a lacuna e não habilita um botão falso.
- **Continuous Testing e cobertura não existem.** Aparecem desabilitados com motivo, como o debug de teste.
- **NuGet: update em lote e install descoberto foram provados numa cópia descartável da fixture.** A fixture versionada não foi alterada, e o escopo Solution no app real agrega só os projetos que ele conhece.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **`.slnx` abre, mas não restaura nem builda.** O SDK 8.0.408 pinado não suporta `.slnx`; a falha aparece como erro, sem sucesso inventado.
- **Abrir pelo Finder/Dock foi verificado por `open -a`.** Duplo clique com mouse e arrasto real para o Dock não foram exercitados.
- **Rollback e "Commit and Push…" no painel Git aparecem desabilitados com motivo.** O host não expõe essas operações; amend já existia.
- **`Escape` para fechar o menu de contexto foi provado no Chromium.** No app real os menus foram fechados por clique físico; o `AXCancel` do macOS não fecha menu React.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.slnx`, `.csproj` ou pasta).
- Tags 13–16 da paridade ficaram **adiadas**: debug de teste, Add Existing Item, Move e Exclude continuam sem implementação.
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado.
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
1d2a84a1cde4b4ddb17ac4b3bd4b85ec63ec988160b89391f1445bf52fb52419  OpenRidr_0.3.0-alpha.9_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: 66,34 MiB (69.559.207 bytes). Verificado no DMG
montado: `CFBundleShortVersionString` 0.3.0-alpha.9, binário Mach-O arm64,
`codesign --verify --deep --strict` e `xcrun stapler validate` no app e no DMG,
`spctl --assess --type execute` no app = *accepted, source=Notarized Developer ID* e
`spctl --assess --type open` no DMG = *accepted, source=Notarized Developer ID*. Os **18 Mach-O
internos** foram conferidos um por um: 17 com Developer ID + hardened runtime + timestamp do time
K6M85697D4 (muxer `dotnet` com `allow-jit` + `disable-library-validation`, 13 dylibs do runtime,
`netcoredbg` + `libdbgshim`) e o `createdump` do runtime assinado pela Microsoft. Notarização
`Accepted` no app (`c528d785…`) e no DMG (`43b9d494…`), ambos grampeados.

O app foi aberto com **quarentena simulada** (`com.apple.quarantine` no DMG, montado, copiado e aberto
por `open`), então rodou translocado como um app baixado de verdade: o macOS pediu a confirmação de
primeira abertura, o app abriu, o **setup foi direto (nenhum download, sem `tools/` no
`OPENRIDR_HOME`)** e o analisador rodou o `csharp-ls` embutido (`hostroot/dotnet …/Contents/Resources/
csharp-ls/CSharpLanguageServer.dll`). Com a fixture `OrderDesk`: trust, `Program.cs` aberto, erro
digitado no editor e diagnosticado ao vivo (`error CS0029`, `LSP · csharp-ls · session 1`), dois ⌘Z
limpando o diagnóstico, e **Build succeeded · exit 0** (0 warnings, 0 erros, 1,41 s) com as DLLs em
`bin/Debug/net8.0`. Uma instância só, fechada ao final; volumes desmontados.

Nesta release, no HEAD `b5d7964` + este commit: `bun run check` OK, `bun run build` OK (dentro do
`tauri build`), `bun test tests/unit` **35/35**, `cargo test -p openridr-host` **406/406** (268 lib
+ 8 bundled_analysis + 14 debugger + 98 host + 10 test_discovery + 7 test_run + 1 trust_model),
sem nenhuma falha. E2E/Playwright não rodaram nesta release (ver limitações).

## Notas de licença

O app distribui `netcoredbg` 3.2.0-1092 (MIT, Copyright © 2017 Samsung Electronics Co., LTD), o
runtime .NET privado 10.0.14 e o `csharp-ls` 0.28.0 (MIT, Copyright © .NET Foundation and
Contributors), componentes Roslyn/dbgshim (MIT), Lucide (ISC e MIT/Feather), Material Design Icons
(Apache-2.0), os glifos de árvore de `@pierre/trees` (Apache-2.0) e o `c-sharp` do Simple Icons
(CC0 1.0), a referência visual do T3 Code (MIT) e as fontes Inter e JetBrains Mono (OFL-1.1). O
texto completo está em `Contents/Resources/THIRD-PARTY-NOTICES.md`.
