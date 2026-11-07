# 6. Atualização dentro do app (updater)

## Sub-features

- Checagem silenciosa ~3 s depois do primeiro paint; falha de rede não interrompe ninguém.
- Item nativo **Check for Updates…** no menu do app (logo abaixo de About), que checa na hora e mostra
  "já é a mais nova", "não disponível neste build" ou o erro.
- Modal `update-dialog`: versão nova e a atual (`update-version`), notas roláveis (`update-notes`),
  **Download and install** (`update-install`) / **Later** (`update-later`).
- Barra de progresso (`update-progress`) e, ao terminar, **Restart now** (`update-restart`) /
  **Later**. "Restart now" passa pelo prompt de buffer não salvo do ⌘Q.
- Erro de instalação em `update-error` (ex.: assinatura inválida), com **Try again**.
- Fora de um `.app` (`tauri dev`, ponte HTTP) a checagem responde `unsupported` e o modal diz que
  este build não se atualiza; sem `__TAURI_INTERNALS__` o componente nem monta.

## How to get to it (user POV)

Abrir o app instalado e esperar ~3 s (o modal abre sozinho quando há versão nova), ou usar
**OpenRidr → Check for Updates…** no menu do app.

## Driving it

- **C** (é a única superfície: o updater só existe no shell desktop). Receita completa em
  `docs/evidence/in-app-updater.md`; o resumo:
  1. Dois builds de release locais com `--config` sobrescrevendo **só**
     `plugins.updater.endpoints` para `http://127.0.0.1:<porta>/latest.json` e
     `dangerousInsecureTransportProtocol: true` (override não commitado). O segundo com a versão do
     `tauri.conf.json` bumpada localmente e revertida depois.
  2. `tar --no-mac-metadata -czf OpenRidr.app.tar.gz OpenRidr.app` + `tauri signer sign
     --app-version <versão>` (a chave real está em `~/.tauri/openridr-updater.key`, com senha em
     `~/.tauri/openridr-updater.password`).
  3. Servidor em loopback entregando o `.tar.gz` em blocos (senão o progresso passa em 1 s) e o
     `latest.json` com os campos de `scripts/release-updater-manifest.sh`.
  4. `desktop.sh up <bundle>` com `VERIFY_OPENRIDR_DESKTOP_STATE` próprio; dirigir com
     `agent-desktop` (`act.py` da skill `test-app-agent-desktop`).
- **Menu**: `agent-desktop snapshot --surface menubar --app OpenRidr` e clicar `Check for Updates…`.
- **Estado final observável**: o modal diz "OpenRidr <nova> is installed"; depois de "Restart now" →
  "Discard and close app" o PID muda, o `Info.plist` do bundle em disco tem a versão nova e o menu
  **Check for Updates…** responde "OpenRidr <nova> is the newest version." (a versão **em execução**
  vem de `package_info().version`, compilada no binário).
- **Prova negativa**: manifesto anunciando a versão seguinte com um caractere da assinatura alterado
  → "Could not install the update: …" e o bundle/PID não mudam.

## Gotchas

- O `tar` do macOS grava `._OpenRidr.app` para os xattrs do bundle e o leitor tar do plugin **não**
  funde essa entrada: o install morre em `failed to unpack '._OpenRidr.app'`. Use
  `tar --no-mac-metadata`. `tar -tzf` não mostra o problema (o bsdtar esconde AppleDouble);
  `pax -r` mostra.
- O endpoint e o `dangerousInsecureTransportProtocol` ficam **compilados** no binário: mudar o
  servidor exige rebuild, não é config de runtime.
- `open -n` (o relançamento do `AppHandle::restart()`) **herda** o ambiente do processo que chamou,
  então `OPENRIDR_HOME` isolado sobrevive ao restart.
- O download acontece antes da verificação da assinatura: a recusa custa a transferência inteira.
- Build ad-hoc perde permissões de TCC a cada troca de bundle; não conclua nada sobre TCC a partir
  de uma prova ad-hoc.
