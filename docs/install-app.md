# Instalar o OpenRidr como app local (macOS ARM64)

Uso pessoal, sem assinatura nem notarização.

```sh
bun run app:install
```

O comando faz `tauri build --bundles app` e copia o bundle para `/Applications/OpenRidr.app`, substituindo a cópia anterior. Para atualizar, rode de novo depois de subir a versão em `package.json` e `src-tauri/tauri.conf.json`.

- **Assinatura:** ad-hoc (`signingIdentity: "-"`), necessária para o Apple Silicon executar o binário. Não é Developer ID.
- **Gatekeeper:** o app não é notarizado. Na primeira abertura, use botão direito → Abrir, ou remova a quarentena com `xattr -dr com.apple.quarantine /Applications/OpenRidr.app`.
- **Ícone:** provisório, o mesmo `src-tauri/icons/icon.png` do repositório, convertido para `.icns`/`.ico`.
- **Fora deste fluxo:** assinatura de distribuição, notarização, instalador público e Windows/macOS Intel.
