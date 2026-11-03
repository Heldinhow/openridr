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

## Abrir do Finder e do terminal

Nada disto é instalado automaticamente: nem o app nem o `install-app.sh` mexem no PATH, no shell
ou em links. O app também não é notarizado, então vale a mesma nota de Gatekeeper acima.

- **Abrir Com:** o bundle declara `.sln`, `.slnx`, `.csproj` (Editor, padrão) e pastas (Alternate,
  não vira padrão para pastas). Depois de instalar, use Finder → Obter Informações → Abrir com →
  OpenRidr → Alterar Todos para tornar o app padrão de um tipo.
- **Arrastar pasta para o Dock:** arraste a pasta para o ícone do OpenRidr no Dock. O app abre a
  solution ou o projeto que ela contém; com duas solutions, a escolha é explícita.
- **Rebuild obrigatório:** as associações vêm do `Info.plist` gerado. Depois de mudar
  `tauri.conf.json`, gere de novo com `bun run app:install` (ou
  `bun run tauri build --debug --bundles app` para o bundle de debug em
  `src-tauri/target/debug/bundle/macos/OpenRidr.app`).
- **Cache do LaunchServices:** se o Finder ainda não mostrar o app, registre o bundle de novo:
  `/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/OpenRidr.app`
  e, se preciso, reabra o Finder.
- **Terminal:** `scripts/openridr <caminho>` valida que o caminho existe e chama
  `open -a OpenRidr <caminho>`, que reusa a instância já aberta. Uso inválido ou caminho ausente
  saem com erro em `stderr` e código diferente de 0.
- **Link manual no PATH (opcional, sua decisão):**

  ```sh
  mkdir -p ~/.local/bin && ln -s "$PWD/scripts/openridr" ~/.local/bin/openridr
  ```

  Ou `/usr/local/bin`, se ele já estiver no seu PATH. O link aponta para este repositório: se o
  repositório mudar de lugar, refaça o link.
- **Dev, bundle de debug:** `OPENRIDR_APP` aponta o launcher para outro bundle:

  ```sh
  OPENRIDR_APP="$PWD/src-tauri/target/debug/bundle/macos/OpenRidr.app" scripts/openridr fixtures/OrderDesk/OrderDesk.sln
  ```

  O bundle de debug usa o mesmo `CFBundleIdentifier` do app instalado. Se um deles já estiver
  aberto, o LaunchServices reusa essa instância, não a outra. Feche a instância antes de testar.
  `open` também não repassa variáveis do shell: para isolar `OPENRIDR_HOME`, use
  `open --env OPENRIDR_HOME=<dir> -a <bundle> <arquivo>` direto, sem o launcher.
