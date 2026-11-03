# OpenRidr 0.3.0-alpha.5 (pré-release)

Quinta alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.

## Novidades

- **Abrir solution do Finder, do Dock e do terminal.** O app se registra em `.sln`, `.slnx` e `.csproj` (Editor, padrão): duplo clique no Finder ou **Abrir Com → OpenRidr** abre o projeto, com o app fechado ou já aberto. Arrastar uma pasta para o ícone do OpenRidr no Dock abre a solution ou o projeto que ela contém; com duas solutions, a escolha é explícita. No terminal, `scripts/openridr <caminho>` faz o mesmo por `open -a`. A abertura externa entra pelo mesmo `workspace.open` do "Abrir solução": trust, recentes, candidatos ambíguos e recusa de caminho continuam no host, a instância aberta é reusada (sem janela nova) e buffer com edições não é descartado.
- **O app agora é assinado e notarizado.** O DMG sai com **Developer ID Application** e hardened runtime, o `.app` e o DMG carregam o ticket de notarização da Apple grampeado e o DMG também é assinado. A primeira abertura **não precisa mais** de `xattr` nem de botão direito: arraste para **Applications** e abra. Vale só para este app distribuído; o build local (`bun run app:install`) continua ad-hoc.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.5_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra normalmente. Gatekeeper aceita sem diálogo: assinatura Developer ID + notarização estão no app e no DMG.
3. É preciso um .NET SDK instalado (o app só detecta, nunca instala).
4. Para virar o app padrão de `.sln`/`.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr → **Alterar Todos**.

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.

## Limitações conhecidas

- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto, e o host só faz `launch` de assembly console; o produto informa a lacuna e não habilita um botão falso.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **`.slnx` abre, mas não restaura nem builda.** O outline lê o arquivo; restore e build falham porque o SDK 8.0.408 pinado não suporta `.slnx`, e a falha aparece como erro, sem sucesso inventado.
- **O app Tauri real foi pouco testado.** A janela não foi dirigida por muito tempo; a suíte E2E roda no Chrome contra a ponte HTTP de dev, não na janela WKWebView nativa. Persistência entre reinícios reais do app, ⌘O nativo (só Ctrl+O na ponte) e leitor de tela (VoiceOver) não foram verificados.
- **Abrir pelo Finder/Dock foi verificado por `open -a`.** Duplo clique com mouse e arrasto real para o Dock não foram exercitados; a recepção do caminho, a instância única e as associações no `Info.plist` foram.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.slnx`, `.csproj` ou pasta).
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado.
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
SHA256  OpenRidr_0.3.0-alpha.5_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: TAMANHO. Verificado no DMG montado:
`CFBundleShortVersionString` 0.3.0-alpha.5, binário Mach-O arm64, `codesign --verify --deep --strict`
no app, `spctl --assess --type execute` = *accepted, source=Notarized Developer ID*, 
`spctl --assess --type open` no DMG aceito e `xcrun stapler validate` verde no app e no DMG.
