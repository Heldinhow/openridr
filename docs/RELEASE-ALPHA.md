# OpenRidr 0.3.0-alpha.11 (pré-release)

Décima primeira alfa instalável da IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS).

## Novidade desde a alpha.10

- **Perfis de execução (#40).** Run e Debug importam perfis de `Properties/launchSettings.json`.
  O seletor mostra `Projeto · perfil`; `applicationUrl` fornece `ASPNETCORE_URLS`, e variáveis de
  ambiente, argumentos e diretório de trabalho são respeitados. Perfis IIS e Docker são listados
  como não suportados.
- O analisador e o depurador continuam embutidos; o SDK do projeto permanece externo.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.11_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
   Arraste antes de abrir: aberto direto da janela do DMG, o app leva ~5–7 s até a janela depois do
   clique, porque o macOS lê os 164 MB de dentro da imagem comprimida.
2. Abra o app. Na primeira abertura o macOS pergunta se você quer abrir um app baixado da internet:
   **clique em Abrir** (ou tecle Enter). O diálogo aparece em ~0,5 s e a janela em ~1,4 s depois do
   clique (medido nesta máquina, no alpha.11); o tempo parado antes disso é a espera do clique, não
   verificação do app. App e DMG têm assinatura Developer ID e ticket de notarização. Nas aberturas
   seguintes não há diálogo.
3. Instale separadamente o **.NET SDK que o projeto pede**. O OpenRidr apenas o detecta: não instala
   SDK nem modifica `global.json`.
4. Para associar `.sln` ou `.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr →
   **Alterar Todos**.

## Requisitos

- macOS 13 ou superior em **Apple Silicon (arm64)**. Intel e Windows não foram verificados.
- O SDK .NET exigido pelo projeto. O runtime privado usado pelo analisador já vem no bundle.

## Limitações conhecidas

- O tema claro não está disponível; o tema distribuído é escuro.
- A suíte E2E completa tem falhas e instabilidades conhecidas e não foi executada nesta release.
- Atualizar o analisador ou netcoredbg exige outra release. O hash exibido no setup é informativo;
  a integridade da distribuição depende da assinatura e notarização.
- Dock Quit e AppleScript `quit` não perguntam sobre alterações não salvas; os caminhos próprios da
  janela/menu do app oferecem a confirmação.
- Debug de teste, Continuous Testing, cobertura, Format, Organize Usings, New e Clone não estão
  implementados. Rollback e Commit and Push aparecem desabilitados com motivo.
- `.slnx` abre, mas não restaura nem builda com o SDK 8.0.408 da fixture. O tipo web é detectado
  pelo texto do `.csproj`, sem fixture web que prove a tela.
- Find in Files não oferece regex ou máscara; Recent Files e Back/Forward são da sessão.
- O build é desabilitado se a avaliação de projeto falhar. O SDK instalado deve atender ao projeto.

## Verificação

O corpo publicado desta release é montado pelo workflow a partir deste arquivo: nome do DMG,
SHA-256, tamanho em bytes e o resultado de `codesign`, `spctl` e `stapler` do run entram no bloco
`## Verificação` gerado no fim ([release-signing.md](release-signing.md)). Um hash escrito à mão não
entra, porque ele só pode estar errado: quem confere usa o hash do próprio run.

App e DMG são assinados com Developer ID e grampeados; `codesign` e `spctl` aceitam o app, inclusive
na imagem montada, e `spctl` aceita o DMG como *Notarized Developer ID*.

No smoke desktop com cópia do DMG em quarentena e `OPENRIDR_HOME` isolado, o setup abriu sem
download, Problems mostrou `CS0029`, Build terminou com exit 0 e o perfil `http` imprimiu
`ASPNETCORE_URLS=http://localhost:5083`. Não foi observado o diálogo gráfico de primeira abertura.

`bun run check` e 35 testes unitários passaram. A execução completa do host teve uma falha de
integração que passou isoladamente; as suítes restantes passaram separadamente. Sem E2E/Playwright.
Detalhes e limites: [evidence/release-alpha11.md](evidence/release-alpha11.md).

## Notas de licença

O app inclui netcoredbg, runtime .NET, csharp-ls, Roslyn/dbgshim, Lucide, os SVGs do Material Icon
Theme, e fontes Inter e JetBrains Mono. Licenças e atribuições completas estão em
`Contents/Resources/THIRD-PARTY-NOTICES.md`.
