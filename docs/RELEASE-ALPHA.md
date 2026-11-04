# OpenRidr 0.3.0-alpha.6 (pré-release)

Sexta alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.
Esta alfa fecha os 20 achados do E2E da alpha.5, corrige a camada de SDK/toolchain e passa a
distribuir o depurador dentro do app.

## Novidades

- **Os 20 achados do E2E da alpha.5.** Sair com **⌘Q** ou pelo menu Quit agora pergunta antes de fechar com alterações não salvas (Save all / Keep editing / Discard and close app); o botão Close continua igual. **Debug** deixa de bloquear depois de apagar um `.cs` por fora e rebuildar verde. Aba limpa recarrega sozinha quando o arquivo muda no disco, a árvore reflete criar/renomear/apagar externo e ganhou **criar, renomear e apagar** com confirmação, confinado ao workspace. Salvar preserva **CRLF** e **UTF-16/BOM**. Mensagens de abertura e avaliação dizem a verdade (`.sln` sem projetos, TFM inválido na primeira avaliação). Nomes acessíveis de Run/Save e do menu de projeto são inequívocos. Uma segunda execução direta do binário reusa a instância aberta e encaminha o `.sln`.
- **SDK e ferramentas por projeto.** O app não exige mais SDK 10 de um projeto preso ao 8: o SDK vem do `global.json`/TargetFramework, e o csharp-ls exige só o **runtime** .NET 10. A descoberta cobre `DOTNET_ROOT`, `/usr/local/share/dotnet`, `/opt/homebrew`, `~/.dotnet`, os SDKs gerenciados e todas as entradas do PATH. Quando falta algo, a mensagem cita a versão pedida e o link de instalação correto. O download do csharp-ls na primeira abertura de projeto C# tem progresso, retry e erro claro.
- **Depurador embutido.** O `netcoredbg` 3.2.0-1092 (osx-arm64) agora vai dentro do app, assinado com Developer ID + hardened runtime e coberto pela notarização. O primeiro Debug **não baixa mais nada**; o adaptador é resolvido de `Contents/Resources` e há fallback para uma instalação anterior. Workspace tools mostra "Bundled with OpenRidr", versão e SHA-256.
- **Tamanho.** O DMG cresce de 5,41 MiB para **9,97 MiB**; o app instalado de 14,68 MiB para 23,85 MiB. O `THIRD-PARTY-NOTICES.md` com a licença MIT do netcoredbg e dos componentes Roslyn/dbgshim vai dentro do app.
- **Fechamento da alpha.5.** Abrir solution pelo Finder/Dock/terminal, assinatura Developer ID + notarização do app e do DMG, Solution Explorer no estilo Rider, Build Solution, Find in Files, Recent Files, navegação e debug com build automático continuam como na alpha.5.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.6_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra normalmente. Gatekeeper aceita sem diálogo: assinatura Developer ID + notarização estão no app e no DMG.
3. É preciso o .NET SDK que o projeto pede (o app só detecta, nunca instala nem edita `global.json`). A análise C# baixa o csharp-ls (~99 MB) na primeira abertura de projeto, com progresso e retry.
4. Para virar o app padrão de `.sln`/`.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr → **Alterar Todos**.

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.
- **.NET SDK** do projeto (o app não instala). A análise usa o **runtime .NET 10** (o csharp-ls o exige); o app não exige SDK 10 de um projeto que pede 8.

## Limitações conhecidas

- **Dock Quit e AppleScript `quit` não perguntam.** Esses caminhos usam `terminate:` nativo do macOS, que o Tauri não expõe; ⌘Q, o menu Quit e o botão Close perguntam. Cobertura por `applicationShouldTerminate:` via objc/unsafe fica para uma decisão futura.
- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto; o produto informa a lacuna e não habilita um botão falso.
- **Atualizar o netcoredbg exige release do app.** A versão é fixada e assinada com o app; não há update independente da ferramenta.
- **O csharp-ls continua sendo baixado pelo usuário.** Embutir a análise exigiria carregar o runtime .NET junto, fora do escopo.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **`.slnx` abre, mas não restaura nem builda.** O SDK 8.0.408 pinado não suporta `.slnx`; a falha aparece como erro, sem sucesso inventado.
- **Abrir pelo Finder/Dock foi verificado por `open -a`.** Duplo clique com mouse e arrasto real para o Dock não foram exercitados.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.slnx`, `.csproj` ou pasta).
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado.
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
PENDENTE  OpenRidr_0.3.0-alpha.6_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: **PENDENTE**. Verificado no DMG montado:
`CFBundleShortVersionString` 0.3.0-alpha.6, binário Mach-O arm64,
`codesign --verify --deep --strict` e `xcrun stapler validate` no app e no DMG,
`spctl --assess --type execute` no app = *accepted, source=Notarized Developer ID* e
`spctl --assess --type open` no DMG = *accepted, source=Notarized Developer ID*. O Debug foi
executado a partir do DMG somente leitura, com `OPENRIDR_HOME` isolado e sem `tools/netcoredbg`,
imprimindo a saída do debuggee.

## Notas de licença

O app distribui `netcoredbg` 3.2.0-1092 (MIT, Copyright © 2017 Samsung Electronics Co., LTD) e
componentes Roslyn/dbgshim (MIT). O texto completo está em `Contents/Resources/THIRD-PARTY-NOTICES.md`.
