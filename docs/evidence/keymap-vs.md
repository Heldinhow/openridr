# Keymap Visual Studio — evidência

Em 2026-11-06, app Tauri debug ad-hoc no macOS ARM64, uma instância, `OPENRIDR_HOME` e cópia da fixture OrderDesk isolados em `/tmp`. Dirigido com agent-desktop, sem Playwright.

- Settings → Keymap → Visual Studio → Save; o arquivo de configurações isolado registrou `keymap.profile: visual-studio`. Reabertura do painel confirmou a seleção: `keymap-vs/04-configuracao.png`.
- Status bar e dica do editor mostraram Visual Studio, F12, F2 e Cmd+.: `keymap-vs/01-perfil-editor.png`.
- F9 em Program.cs marcou o breakpoint na linha 1: `keymap-vs/02-breakpoint.png`.
- F12 sobre `Order` em Program.cs abriu a definição em Order.cs: `keymap-vs/03-definicao.png`. A análise informava cobertura parcial; a navegação específica foi observada.
- Não verificados no app real: execução e depuração dos demais atalhos, reinício do app para conferir persistência, Windows e macOS Intel. O host verificou o round-trip de configurações em teste unitário.
