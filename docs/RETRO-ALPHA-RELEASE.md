# Retro — release alfa 0.3.0-alpha.1

- **Ícone:** `bunx tauri icon` baixa o pacote errado (`tauri`, compila `sharp`); funciona com o CLI do projeto (`bun run tauri icon`). O comando também gera Android/iOS/Square*: removidos, ficaram só os arquivos que o projeto já usava.
- **`bundle.icon` ausente:** sem a chave, o `.app` não levaria ícone nenhum, mesmo com `icons/` preenchido. Adicionada.
- **DMG:** o `bundle_dmg.sh` roda AppleScript no Finder e estoura timeout (-1712) sem permissão de automação; `CI=true` pula essa etapa (sem layout de janela customizado). Documentado em `decisions.md`.
- **Verificado:** DMG montado, `OpenRidr.app` arm64, assinatura ad-hoc, `icon.icns` idêntico ao do repo, versão no Info.plist, processo sobe por ~6 s. `bun run check` passa.
- **Não verificado:** instalação em /Applications com quarentena real (Gatekeeper), uso da janela além do boot, suíte cargo/E2E (não afetadas pela mudança), Windows, Intel.
- **Próximo:** notarização exige Developer ID; automatizar o build do DMG num script/CI.
