# OpenRidr 0.3.0-alpha.1 (pré-release)

Primeira alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro, ícone novo.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.1_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
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

SHA-256 do DMG: veja abaixo, nas notas da release.
