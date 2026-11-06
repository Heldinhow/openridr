# OpenRidr 0.3.0-alpha.10 (pré-release)

Décima alfa instalável da IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS).
Esta versão mantém o analisador e o depurador embutidos da alpha.9 e corrige a confiança TLS do host.

## Novidade desde a alpha.9

- **Certificados do sistema (#38).** O host confia nas raízes TLS do sistema/Keychain além das
  raízes webpki, permitindo conexões HTTPS com CA corporativa confiável no download de ferramentas
  e no NuGet. O proxy por variáveis de ambiente continua desabilitado de propósito. A correção não
  foi validada nesta release contra uma rede corporativa real.
- **Ferramentas embutidas.** `csharp-ls` 0.28.0, runtime privado .NET 10.0.14 e netcoredbg
  3.2.0-1092 acompanham o app; o setup de release não precisa baixá-los. O SDK do projeto continua
  externo e é resolvido conforme o `global.json`.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.10_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. Abra o app. O macOS pode pedir confirmação na primeira abertura de um app baixado; o app e o
   DMG têm assinatura Developer ID e ticket de notarização.
3. Instale separadamente o **.NET SDK que o projeto pede**. O OpenRidr apenas o detecta: não instala
   SDK nem modifica `global.json`.
4. Para associar `.sln` ou `.csproj`: Finder → Obter Informações → **Abrir com** → OpenRidr →
   **Alterar Todos**.

## Requisitos

- macOS 13 ou superior em **Apple Silicon (arm64)**. Não há build nem verificação para Intel ou Windows.
- O SDK .NET exigido pelo projeto. O runtime privado usado pelo analisador já vem no bundle.

## Limitações conhecidas

- O tema claro não está disponível; o tema distribuído é escuro.
- A suíte E2E completa tem falhas e instabilidades conhecidas e **não foi executada** nesta release.
  A recuperação de Problems após SIGSTOP/SIGCONT continua sem prova conclusiva.
- Atualizar o analisador ou netcoredbg exige outra release do app. O hash exibido no setup é
  informativo; a integridade da distribuição depende da assinatura e notarização.
- Dock Quit e AppleScript `quit` não perguntam sobre alterações não salvas; os caminhos próprios da
  janela/menu do app oferecem a confirmação.
- Debug de teste, Continuous Testing, cobertura, Format, Organize Usings, New e Clone não estão
  implementados. Rollback e Commit and Push aparecem desabilitados com motivo.
- `.slnx` abre, mas não restaura nem builda com o SDK 8.0.408 da fixture. O tipo web é detectado
  pelo texto do `.csproj`, sem fixture web que prove a tela.
- Find in Files não oferece regex ou máscara; Recent Files e Back/Forward são da sessão.
- O build é desabilitado se a avaliação de projeto falhar. O SDK instalado deve atender ao projeto.

## Verificação

```text
b106b31672ec387cfbd4fea02cb396fde8a0d4d69dd394eecd72772cac15ece3  OpenRidr_0.3.0-alpha.10_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. O DMG mede **69.623.227 bytes** (66,40 MiB).
O app e o DMG foram aceitos pela Apple e grampeados; `codesign --verify --deep --strict` passou no
app e `spctl` aceitou ambos como *Notarized Developer ID*. O app dentro da imagem montada também
passou na verificação de assinatura e `spctl` e informou a versão 0.3.0-alpha.10.

No smoke pelo app real, uma cópia do DMG recebeu quarentena, foi montada e o app copiado abriu
translocado com `OPENRIDR_HOME` isolado. O setup foi direto, sem diretório `tools/`; Problems mostrou
`CS0029` do `csharp-ls` e o build da fixture `OrderDesk` terminou com **exit 0**, sem warnings ou
erros. A confirmação gráfica inicial do macOS não foi observada nesta execução. A instância foi
fechada e o volume desmontado.

Na base `4758157` com o bump: `bun run check` passou, `bun test tests/unit` passou **35/35** e
`cargo test -p openridr-host` passou **408/408**. E2E/Playwright não foram executados por decisão
explícita. Detalhes e limites da prova em [evidence/release-alpha10.md](evidence/release-alpha10.md).

## Notas de licença

O app inclui netcoredbg, runtime .NET, csharp-ls, Roslyn/dbgshim, Lucide, Material Design Icons,
glifos `@pierre/trees` e Simple Icons, e fontes Inter e JetBrains Mono. As licenças e atribuições
completas estão em `Contents/Resources/THIRD-PARTY-NOTICES.md`.
