# OpenRidr 0.3.0-alpha.3 (pré-release)

Terceira alfa instalável. IDE C#/.NET inspirada no Rider (Tauri 2 + Rust + React/TS), tema escuro.

## Novidades

- **Debug em um clique, sem pergunta.** Clicar em Debug (ou usar o atalho) ativa sozinho a flag experimental do netcoredbg no macOS Apple Silicon, reusa a versão aprovada **3.2.0-1092** quando ela já existe e só instala se faltar. Em seguida o host builda quando não há build utilizável e inicia a sessão. O antigo diálogo "Enable the experimental debugger?" deixou de existir.
- **O status experimental continua visível, sem bloquear.** O painel Debug mostra uma nota discreta (`dap-experimental-note`) lembrando que o build arm64 do netcoredbg é experimental e com suporte da comunidade; nenhum modal interrompe o fluxo.
- **Confiar direto do aviso de bloqueio.** Com Build, Run ou Debug bloqueados por Restricted mode, o próprio motivo mostra **Trust this project**: um clique concede e memoriza a confiança para o projeto, sem passar pela caixa de seleção do popover da barra de status. O indicador de confiança continua apenas na barra de status e no menu "⋯" dos recentes.
- **Restore automático ao confiar.** O workspace Trusted restaura sozinho uma vez ao abrir/confiar; depois disso o restore do host continua explícito e o Debug builda implicitamente quando não há build atual.

## Instalar

1. Baixe `OpenRidr_0.3.0-alpha.3_aarch64.dmg`, abra e arraste **OpenRidr** para **Applications**.
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
PENDENTE  OpenRidr_0.3.0-alpha.3_aarch64.dmg
```

Confira com `shasum -a 256 <arquivo>.dmg`. Tamanho: PENDENTE.
