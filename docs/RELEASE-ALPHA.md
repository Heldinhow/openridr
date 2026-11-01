# OpenRidr 0.3.0-alpha.2 (pré-release)

Segunda alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.

## Novidades

- **Run e Debug agora constroem sozinhos quando não há build.** Clicar em Run (ou no atalho) usa o build atual quando ele existe e, quando não existe, inicia o build normal — com saída em streaming no painel Build e estado **Building** no Run/Debug. Build falho não executa e mantém os diagnósticos no Build. Parar cancela o build e o lançamento pendente; um segundo clique não inicia outro build.
- **Restore continua explícito.** Run e Debug nunca restauram implicitamente: com dependências pendentes, continuam mostrando `restore_required` e a ação de Restore.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.2_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
2. O app tem assinatura **ad-hoc** (sem Apple Developer ID, sem notarização). O Gatekeeper vai bloquear a primeira abertura:
   - botão direito em OpenRidr.app > **Abrir** > **Abrir**; ou
   - `xattr -dr com.apple.quarantine /Applications/OpenRidr.app`
3. É preciso um .NET SDK instalado (o app só detecta, nunca instala).

## Requisitos

- **Somente macOS Apple Silicon (arm64)**, macOS 13 ou superior. Não há build para Mac Intel nem Windows.

## Limitações conhecidas

Não verificadas (sem evidência no ambiente de build):

- Janela Tauri real em uso prolongado: o DMG foi apenas montado e o app aberto por segundos; os testes E2E rodam contra a ponte HTTP de dev, não contra a janela nativa.
- ⌘O nativo (só Ctrl+O na ponte).
- Leitor de tela (VoiceOver/NVDA).
- Windows e macOS Intel.

Funcionalidade ausente: **New** e **Clone** ainda não existem; abra projetos existentes (`.sln`, `.csproj` ou pasta).

## Verificação

```
be4b8633dd83f003071a7f600bd204ed64e49f6a5c60871170dc76f8d4bf2da4  OpenRidr_0.3.0-alpha.2_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: 4,86 MiB.
