# OpenRidr 0.3.0-alpha.4 (pré-release)

Quarta alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.

## Novidades

- **Solution explorer no estilo Rider.** Uma linha por projeto/pasta/arquivo (`C#`, `· console · net8.0`), contagem `· N projects` na solução e nas pastas, pastas de código aninhadas e **Dependencies** expansível com projetos e pacotes. A barra ganhou Show All Files, busca na árvore, locate, expandir/recolher, menu `⋮` e minimizar. Botão direito em projeto/arquivo/pasta abre menu com Edit, Reload Project, Manage NuGet, Find in Files, Copy Path, Properties e Build/Run/Debug usando o projeto clicado; em Restricted, o menu oferece Trust. Show All Files lista `bin`/`obj` e dotfiles como `no index` (não lê `.gitignore`).
- **Os 4 achados altos do QA de fluxo.** A análise (csharp-ls) inicia sozinha ao confiar no workspace — com o plano de instalação quando o launcher ainda falta — e volta a iniciar em sessões seguintes. Erros de compilação aparecem sublinhados no editor e na margem, além do Problems. A flag `*` de "modified" volta ao limpo depois de undo quando o conteúdo é igual ao do disco. Projeto com avaliação falha deixa de esconder os arquivos na árvore e publica o erro em Problems.
- **Debug sem perder a linha.** O editor rola até a linha de execução atual na parada, no step e na exceção, sem roubar o foco. O keymap global passou a ser capturado antes do editor: `⌘U`+letra (testes), `⇧⌘K` (Push) e `⌃D` (Debug) funcionam com o cursor no código e não alteram mais o buffer.
- **Mais de um console na solução.** O startup project é escolhido direto na toolbar e a escolha é lembrada por solução. **Build Solution (`⇧⌘F9`)** compila o `.sln` inteiro, publica problemas de todos os projetos e o build resultante é reutilizado por Run/Debug; `⌘F9` continua sendo o build do startup project.
- **Navegação e busca.** **Find in Files (`⇧⌘F`)** no workspace ou no escopo do arquivo/pasta/projeto clicado, com Match case, resultados agrupados por arquivo e uma entrada por linha. **Go to Line (`⌘G`/`⌘L`)** funciona dentro e fora do editor. Back/Forward com histórico real (`⌘[`/`⌘]` e `⌘⌥←`/`⌘⌥→`), fechar aba (`⌘W`), próxima/anterior (`⇧⌘]`/`⇧⌘[`, `⌃Tab`/`⌃⇧Tab`), **Recent Files (`⌘E`)** e Go to File (`⇧⌘O`). O perfil **Rider (macOS)** ganhou `⌘D` (duplicar linha) e `⌘⌫` (apagar linha); `⌘T` deixou de ser Search Everywhere nesse perfil. O duplo Shift só abre o Search Everywhere com duas batidas limpas; Push/Update por atalho focam o diálogo e fecham com Esc; Search Everywhere não duplica linhas, prioriza nome exato e reabre com a consulta selecionada.
- **Debug, run e build mais legíveis.** Crash por sinal aparece como **Run crashed · SIGABRT**, em vermelho, com a pilha do log clicável. Erro de compilação da descoberta de testes vai para o Problems. Passar o mouse numa variável durante a pausa mostra o valor. Clicar num frame reutiliza a aba do arquivo em vez de abrir duplicatas. Build de biblioteca e projeto de testes funciona pelo menu da árvore; Run/Debug continuam apenas para consoles. Debug de teste segue indisponível, agora com mensagem explícita ("needs a test-host attach the debugger does not support yet").
- **Editor e estado.** Reabrir a solução restaura abas, arquivo ativo, cursor e breakpoints (breakpoints só com trust). Duplo clique na árvore abre uma única aba. Buffer limpo recarrega do disco quando o arquivo muda por fora; buffer com edições continua mostrando conflito e nunca é sobrescrito. Janela pequena mantém ao menos 180 px de editor. Problems abre com 320 px e contagens consistentes com a barra de status. NuGet mostra a versão resolvida em Current. Projeto com SDK ausente explica o que instalar, oferece **Redetect SDKs**, o link oficial e Open `global.json` — nada é instalado automaticamente. A aba Terminal recebe foco ao abrir.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.4_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. O app tem assinatura **ad-hoc** (sem Apple Developer ID, sem notarização). O Gatekeeper vai bloquear a primeira abertura:
   - botão direito em OpenRidr.app > **Abrir** > **Abrir**; ou
   - `xattr -dr com.apple.quarantine /Applications/OpenRidr.app`
3. É preciso um .NET SDK instalado (o app só detecta, nunca instala).

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.

## Limitações conhecidas

- **Debug de teste não está implementado.** Testes rodam via `dotnet test` num testhost sem pid exposto, e o host só faz `launch` de assembly console; o produto informa a lacuna e não habilita um botão falso.
- **Format e Organize Usings não existem.** O host não expõe `textDocument/formatting` nem `source.organizeImports` do csharp-ls.
- **O app Tauri real foi pouco testado.** O DMG é montado e aberto por segundos; a suíte E2E roda no Chrome contra a ponte HTTP de dev, não na janela WKWebView nativa. Persistência entre reinícios reais do app, ⌘O nativo (só Ctrl+O na ponte) e leitor de tela (VoiceOver) não foram verificados.
- **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.csproj` ou pasta).
- Find in Files não tem regex nem máscara de arquivo; Recent Files e o histórico Back/Forward são da sessão (não persistem). Condições de breakpoint não são restauradas, só a linha. Restaurar sessão não recupera conteúdo não salvo.
- Com avaliação de projeto falha, o Build continua desabilitado (a árvore e o Problems foram corrigidos).
- Windows e macOS Intel não são suportados nem verificados.

## Verificação

```
PENDENTE  OpenRidr_0.3.0-alpha.4_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: PENDENTE.
