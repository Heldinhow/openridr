# SPEC — Atualização dentro do app pelo updater do Tauri (issue #55)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#55](https://github.com/Heldinhow/openridr/issues/55). Decisão registrada em [decisions.md](decisions.md) (2026-11-07).

## Problema

Não existe atualização dentro do app: quem instalou uma alpha só sobe de versão baixando o DMG novo à mão, e nada avisa que existe versão nova. A release publica `OpenRidr_<versão>_aarch64.dmg` assinado e notarizado, mas o app não tem como descobrir isso nem como se substituir.

## Solução

O app passa a checar um manifesto assinado publicado junto das releases, avisar que existe versão nova e, depois do usuário confirmar, baixar o `.app` novo, verificar a assinatura do artefato, trocar o bundle e reiniciar — sem DMG, sem arrastar para `/Applications`.

O manifesto (`latest.json`) e o artefato (`OpenRidr.app.tar.gz` + `.sig`) saem do **mesmo** caminho de release que já existe (`scripts/release-macos.sh` + `.github/workflows/release.yml`): o app atualizado é o app notarizado e grampeado que o DMG carrega, não um segundo build.

## User stories

1. Como pessoa que instalou uma alpha, quero que o app descubra sozinho que existe versão nova, para não precisar visitar a página de releases.
2. Como pessoa que instalou uma alpha, quero ver a versão nova e as notas antes de decidir, para saber o que estou instalando.
3. Como pessoa que instalou uma alpha, quero confirmar antes de baixar, para o app não trocar sozinho o que está rodando.
4. Como pessoa que instalou uma alpha, quero ver o progresso do download, para saber que a atualização está viva.
5. Como pessoa que instalou uma alpha, quero que a atualização preserve o que está aberto, para não perder buffer não salvo ao reiniciar.
6. Como pessoa que instalou uma alpha, quero escolher reiniciar agora ou depois, para terminar o que estava fazendo.
7. Como pessoa que instalou uma alpha, quero que o app recuse um artefato sem assinatura válida, para um servidor comprometido não conseguir instalar outra coisa.
8. Como pessoa que instalou uma alpha, quero que o app recuse um manifesto que anuncia versão diferente da assinada, para um manifesto adulterado não me empurrar uma versão antiga.
9. Como pessoa que instalou uma alpha, quero que uma falha de rede ao checar não vire ruído, para o app não me interromper a cada abertura.
10. Como pessoa que instalou uma alpha, quero poder checar de novo pelo menu, para tentar outra vez depois de dizer "Later".
11. Como pessoa que instalou uma alpha, quero que a atualização funcione atrás do proxy corporativo, para não depender de rede aberta.
12. Como pessoa que instalou uma alpha, quero que o app atualizado continue assinado e notarizado, para o Gatekeeper não bloquear a próxima abertura.
13. Como pessoa que mantém o OpenRidr, quero que o artefato de update saia do mesmo script da release, para não existirem dois caminhos que possam divergir.
14. Como pessoa que mantém o OpenRidr, quero que o artefato de update seja verificado (código, notarização, grampo) antes de ser publicado, para não entregar por update o que o DMG não entregaria.
15. Como pessoa que mantém o OpenRidr, quero que a chave privada de assinatura nunca esteja no repositório, para um clone não conseguir assinar um update.
16. Como pessoa que mantém o OpenRidr, quero que o build de dev e a ponte HTTP não tenham updater, para `tauri dev` não tentar substituir um binário que não é bundle.
17. Como pessoa que valida a release, quero a prova de um update real entre dois builds locais assinados, para saber que o caminho funciona no binário e não só no teste.

## Implementation decisions

- **Fonte do manifesto: release-canal de tag fixa.** O workflow publica o `latest.json` na release `updater-alpha` (pré-release, substituído com `--clobber` a cada versão). O endpoint no `tauri.conf.json` é `https://github.com/Heldinhow/openridr/releases/download/updater-alpha/latest.json`. O **artefato** fica na release da versão (`releases/download/v<versão>/OpenRidr.app.tar.gz`), cuja URL é imutável: substituir o manifesto é o único passo que um cliente observa, e ele acontece depois de a release que tem os bytes existir. Motivo: toda release do projeto é **pré-release**, e `releases/latest/download/latest.json` só resolve a última release não-pré-release. Alternativas recusadas: consultar a API do GitHub em runtime (código novo, 60 req/h sem token) e publicar as alphas como não-pré-release (muda o significado da release).
- **Um canal só.** O app segue a versão mais nova que o workflow publicou no canal (alpha hoje). Sem canal stable/beta separado, sem escolha na UI, sem downgrade: a comparação é a do próprio plugin (`update > current`, semver), então `0.3.0-alpha.12 > 0.3.0-alpha.11` e `0.3.0 > 0.3.0-alpha.12` quando a estável sair.
- **`requireSignedVersion: true`.** O plugin 2.14 recusa um manifesto cuja `version` não é a versão gravada no *trusted comment* da assinatura. Sem isso, um manifesto adulterado pareia versão inflada com o `url`/`signature` de uma release antiga genuína (downgrade forçado). Não há usuário do updater antes desta frente, então não existe release legado sem versão assinada para rejeitar; a assinatura local passa `--app-version` para gravar a versão.
- **Uma dependência Rust, zero dependência JS.** `tauri-plugin-updater` 2.14.1 (casa com `tauri` 2.12.5 e `reqwest` 0.13.5 que já estão no lock, sem reqwest duplicado). Nada de `@tauri-apps/plugin-updater` nem `@tauri-apps/plugin-process`: o fluxo inteiro é Rust e o reinício é `AppHandle::restart()` do Tauri. Nada de `tauri-plugin-dialog`: a confirmação é a UI do app.
- **Três comandos no shell, num módulo próprio.** `src-tauri/src/update.rs` expõe `update_check`, `update_install` e `restart_app`; a UI chama por `invoke`, como já faz com `quit_app`/`take_pending_open`. O host (`crates/host`) não participa: o plugin precisa do `AppHandle`, e a checagem é chamada de rede do shell, não código do projeto — **não** passa por trust/restricted mode.
- **Gate de build empacotado.** `update_check` devolve `unsupported` quando o executável não está em `<Algo>.app/Contents/MacOS/`. É o que separa `tauri dev` e a ponte HTTP de dev (binário solto em `target/debug`) do app instalado; sem o gate, o plugin tentaria trocar um diretório que não é bundle. A decisão é uma função pura sobre o caminho do executável, com teste.
- **Sem prazo total; só teto de conexão.** O timeout do plugin vale para a requisição **e** para o download do artefato (o mesmo `Update` carrega o valor), então um prazo total mataria um download legítimo de ~70 MB em link lento. Fica `connect_timeout` de 15 s (a conexão que nunca abre falha) e nenhum prazo total. Limite declarado: um servidor que aceita a conexão e para de mandar bytes deixa a checagem pendente (no startup, invisível; no menu, "Checking…") e o download parado no último percentual; não há botão de cancelar nesta frente, e fechar o app cancela.
- **Confirmação na UI, com progresso.** Modal com versão, notas (roláveis) e as ações **Download and install** / **Later**; durante o download, barra de progresso com o tamanho (que só existe quando o `Content-Length` chega — antes disso não há número honesto a mostrar); ao terminar, **Restart now** / **Later**. O progresso vem por evento `openridr-update-progress` com `{ downloaded, total }`, emitido no máximo a cada 200 ms e uma vez no fim (o download é de ~63 MB). Depois do último evento a UI mostra "Installing…" até o comando resolver.
- **Uma operação de update por vez, e a versão confirmada é a que instala.** A UI não inicia outra checagem enquanto uma está em voo ou um download/instalação está em andamento (uma resposta atrasada trocaria a versão que o usuário está lendo), e `update_install` recebe a versão mostrada e recusa instalar outra.
- **Checagem automática no startup e pelo menu.** Uma checagem silenciosa ~3 s depois do primeiro paint (falha de rede não abre diálogo e deixa uma linha no stderr do shell), e o item nativo **Check for Updates…** no menu do app emite `openridr-update-check` para a UI checar na hora e mostrar o resultado (inclusive "You're up to date" e o erro, quando houver). Sem retry automático e sem banner: quem diz "Later" tem o menu para tentar de novo.
- **Reiniciar reaproveita o fluxo de quit.** `restart_app` não reinicia na hora: o "Restart now" passa pela mesma decisão de buffer não salvo que o ⌘Q (`restartOnQuit` na UI; o `quit_app` final vira `restart_app`). `restart_app` fecha o workspace do host, marca a saída como aprovada e chama `AppHandle::restart()` — que no macOS relança pelo LaunchServices (`open -n`), então o processo novo lê o bundle já trocado. `app.restart()` direto descartaria buffer não salvo em silêncio.
- **A assinatura do artefato é conferida com a chave que o app confia.** Depois de assinar, o release compara o *key id* minisign da assinatura com o da `pubkey` do `tauri.conf.json` e recusa a release quando diferem. Sem isso, um secret registrado com outra chave publicaria um artefato que todo app instalado rejeita, com o DMG perfeitamente válido. É conferência de **identidade** (o key id é o que identifica a chave), não verificação criptográfica: essa acontece no app, na instalação, e é o que a prova de ponta a ponta mede.
- **Repetir a release de uma versão já publicada tem custo declarado.** O artefato é re-uploadado com `--clobber` (a mesma política do DMG), então entre o upload e a troca do manifesto o manifesto antigo aponta para bytes novos com a assinatura antiga: quem checar naquela janela baixa e recusa. A janela é de segundos no caminho feliz; se o run falhar no meio, o update daquela versão fica recusado até o próximo release. É o preço de poder republicar uma tag, e não uma brecha: a assinatura continua sendo o que decide o que instala.
- **Artefato de update depois da notarização.** `release-macos.sh` cria `OpenRidr.app.tar.gz` a partir do `.app` **já grampeado** (o `.app` com `OpenRidr.app/` na raiz, que é o que o plugin espera), assina com `tauri signer sign --app-version <versão>` e verifica extraindo o arquivo e rodando `codesign --verify --deep --strict`, `spctl --assess --type execute` e `xcrun stapler validate` no app extraído. Alternativa recusada: `bundle.createUpdaterArtifacts: true`, que arquiva no build, **antes** da notarização — o update entregaria um app notarizado mas sem ticket grampeado (primeira abertura offline poderia falhar), e o caminho do update divergiria do DMG. O DMG continua exatamente como está.
- **Manifesto gerado por script próprio.** `scripts/release-updater-manifest.sh <versão> <url> <arquivo .sig> <notas>` imprime o `latest.json` (version, notes, pub_date RFC 3339, `platforms.darwin-aarch64.{signature,url}`) e falha em versão fora de semver, URL não-https, assinatura vazia ou fora do formato minisign. `notes` é o conteúdo de `docs/RELEASE-ALPHA.md` (as notas humanas, sem o bloco `## Verificação` que o `release-notes.sh` gera).
- **Chave de assinatura fora do repo.** Par gerado fora do repositório, **com senha**; só a pública entra no `tauri.conf.json` (é pública por construção). O workflow recebe `TAURI_SIGNING_PRIVATE_KEY` (conteúdo da chave) e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; o preflight do `release-macos.sh` exige os dois nomes (nomes apenas, nunca valores) e o `release-selfcheck.sh` ganha o caso de ausência. Perder a chave ou a senha significa que nenhum update futuro funciona.
- **Proxy e TLS herdados do reqwest.** O cliente do plugin usa `rustls-platform-verifier` (raízes do Keychain, então CA corporativa instalada funciona) e `system-proxy` (variáveis de ambiente **e** os Proxies do System Settings do macOS). A guarda de endereço do host (`netguard`) não se aplica: o updater só fala com github.com e com o host do artefato publicado na mesma release. Sem ponte, sem configuração nova.
- **Publicação em ordem.** O workflow publica a release da versão (DMG, artefato de update e assinatura) primeiro e troca o `latest.json` do canal depois — um cliente que lê o manifesto nunca encontra uma URL que ainda não existe, e o manifesto antigo continua apontando para bytes imutáveis.
- **Testids novos, nenhum renomeado.** `update-dialog`, `update-version`, `update-notes`, `update-progress`, `update-install`, `update-later`, `update-restart`, `update-error`, `update-check-result`. Nenhum `data-testid` existente muda e nenhum `role="tab"` é tocado.

## Testing decisions

- **Teste bom aqui prova comportamento observável, não texto de tela.** No Rust, as funções puras: o gate de bundle (caminho dentro de `.app/Contents/MacOS` vs binário solto), a mensagem de erro de cada fase e a decisão de emitir progresso. Na UI, o que dá para provar sem DOM é a máquina de estados do modal; o resto é prova no app real.
- **Seams existentes, nenhum seam novo de host.** O shell já tem `#[cfg(test)] mod tests` em `main.rs` (argv, `file://`); `update.rs` segue o mesmo molde com `cargo test --manifest-path src-tauri/Cargo.toml`. Nada no host muda, então nenhum teste de `crates/host` é tocado.
- **Falha-antes/passa-depois** no gate de bundle: com a checagem de caminho ausente, o teste do binário solto reprova (o plugin tentaria instalar sobre `target/debug`).
- **Scripts com self-check, no molde do `release-notes-selfcheck.sh`**: `release-updater-manifest-selfcheck.sh` cobre versão fora de semver, URL não-https, `.sig` ausente/vazio e o caso válido (as quatro chaves e o `darwin-aarch64`). O `release-selfcheck.sh` ganha o caso da credencial de assinatura ausente.
- **Prova real: um update de ponta a ponta entre dois builds locais.** Build de release local do alpha.11 (ad-hoc, `--config` sobrescrevendo só `plugins.updater.endpoints` para `http://127.0.0.1:<porta>/latest.json` e `dangerousInsecureTransportProtocol: true`), instalado num diretório gravável; um segundo build com a versão do `Info.plist` em alpha.12, re-assinado, empacotado e assinado com a chave real; um servidor local servindo o `latest.json`. O app alpha.11 detecta, mostra a versão nova, confirma, baixa, instala e reinicia mostrando alpha.12. O override **não** é commitado — o `tauri.conf.json` do repo fica só com https.
- **Prova no app Tauri real com `agent-desktop`** para a superfície: modal com versão/notas, progresso e o item de menu. Uma instância por vez, com `OPENRIDR_HOME` isolado.
- **Nada de Playwright**: smoke noturno, não prova de sessão (decisão da #67).

## Out of scope

- **Windows e Linux**: o projeto só foi verificado em macOS ARM64; o plugin suporta os dois, mas nada aqui é provado neles.
- **Delta updates, rollout escalonado, download automático sem confirmação, troca de canal, tela de settings de update, telemetria, "what's new", downgrade.**
- **Cancelar o download em andamento**: fechar o app cancela; um botão exige estado de operação em voo no shell.
- **Atualizar pelo DMG**: o caminho manual continua sendo o DMG da release; o update não o substitui.
- **Trocar a chave pública em apps já instalados**: a chave vale para sempre; rotacionar exige um caminho de migração que não existe.

## Further notes

- O canal `updater-alpha` aparece na lista de releases do repositório como pré-release com um título próprio ("Update channel"); o corpo diz que não é para baixar dali. O DMG continua na release da versão.
- O app atualizado mantém o bundle ID e a identidade de assinatura, então as permissões de TCC (Acessibilidade, Gravação de Tela) concedidas à versão anterior continuam valendo. Um build **ad-hoc** (a prova local) perde essas permissões a cada troca de bundle, porque o requisito de código é o hash — por isso a prova não depende delas.
- A primeira versão que entrega o updater é a primeira que pode ser atualizada por ele: nenhuma release anterior tem `latest.json`.
