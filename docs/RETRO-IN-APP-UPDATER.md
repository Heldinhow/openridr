# RETRO — Atualização dentro do app pelo updater do Tauri (issue #55, tickets #195–#198)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-IN-APP-UPDATER](SPEC-IN-APP-UPDATER.md). Evidência:
[evidence/in-app-updater.md](evidence/in-app-updater.md). Decisão: [decisions.md](decisions.md)
(2026-11-07). Receita de verificação: [features/in-app-updater.md](../.agents/skills/verify-openridr/features/in-app-updater.md).

## O que se sustentou

- **O grill fechou o desenho antes do código.** Treze decisões numa rodada (fonte do manifesto, canal, confirmação, restart, artefato, chave, proxy, escopo). As duas que mais teriam custado caro depois — endpoint em `releases/latest` (que nunca resolve, porque toda release é pré-release) e artefato saído do bundler (que arquiva antes da notarização) — morreram na pergunta, não no build.
- **Ler o código do plugin antes de desenhar.** Três fatos do fonte mudaram o desenho e não estão na documentação: o timeout vale também para o download (daí só `connect_timeout`), o `signature` do manifesto é o `.sig` já em base64, e o extrator descarta o primeiro componente do caminho (daí a raiz única e a guarda da AppleDouble).
- **Leitor independente é o que prova artefato.** O `tar -tzf` mostrava o arquivo limpo porque é o mesmo `bsdtar` que o escreveu: ele esconde a entrada AppleDouble que quebra o consumidor. Quem revelou foi o `pax -r` e o crate `tar` do plugin. Regra que fica: validar um artefato com um leitor da família do consumidor, nunca com o que o escreveu.
- **Prova de ponta a ponta antes do PR.** O update real entre dois builds locais achou o bug da AppleDouble, que nenhum teste unitário, `tsc` ou revisão de diff acharia — o install morria depois de baixar 63 MB.
- **Reaproveitar o fluxo de quit.** O prompt de buffer não salvo apareceu no meio da prova sem código novo de guarda; `restartOnQuit` só escolhe a última chamada.
- **A guarda viva no self-check.** `release-selfcheck.sh` planta o xattr no bundle do fixture e reprova sem `--no-mac-metadata`, com a mensagem exata do defeito. Falha-antes medida, não afirmada.

## O que quebrou / custou

- **O fixture do self-check não reproduzia a condição real.** O stub sem xattr passava com `tar` puro; o bundle de verdade tem `com.apple.provenance`. O bug só apareceu porque a prova foi de ponta a ponta.
- **Duas guardas decorativas antes da que funciona.** `pax -rzf` + `ls` não detectava nada (o pax funde a AppleDouble quando consegue) e `tar --no-xattrs` não muda a escrita. A guarda boa lê o primeiro header do fluxo cru.
- **Diagnóstico perdido por ler o log errado.** O `err` do self-check é reescrito por cada `run_signed`; eu grepei o arquivo do último caso e concluí que a guarda não rodava. Custou uma rodada; resolveria com o `$work` preservado.
- **O doctor rodou depois da prova, não antes.** O `verify-openridr` pede doctor primeiro; pulei e só rodei no fim (tudo `ok`). Ordem errada: um `FAIL` no doctor teria invalidado a prova inteira.
- **O `snapshot -i` do `agent-desktop` não mostrou o modal.** Era o filtro/truncamento do meu extrator, não o produto: `dialog`, `heading` e os botões com `ref_id` estão no snapshot completo. Alguns minutos achando que era defeito de acessibilidade.
- **A versão em execução não tem superfície própria.** O `snapshot.app.version` existe no host e não aparece na UI; a prova leu a versão pelo "Check for Updates…". Um About com a versão é o lugar natural e não existe.

## Onde o ambiente atrapalhou (candidatos, por severidade)

1. **Checagem automática — o doctor não é obrigatório no fluxo.** Nada força `doctor.sh` antes da superfície C; eu pulei. Candidato: uma linha no `verify-openridr` dizendo que a prova sem doctor é inválida, ou o `desktop.sh up` rodando o doctor e abortando em `FAIL`.
2. **Ferramenta — o self-check apaga o `$work` no `trap`.** Depurar um caso exige editar o script. Candidato: `RELEASE_SELFTEST_KEEP=1` mantém o diretório e imprime o caminho.
3. **Informação — `tar -tzf` mente sobre AppleDouble e nada avisa.** Candidato: uma linha em `docs/release-signing.md` (já escrita) e o comentário no script (já escrito); o que faltava era o conhecimento, não a ferramenta.
4. **Navegação — o endpoint do updater é compilado.** Mudar de servidor exige rebuild e não há como ver o endpoint ativo. Candidato: o `update_check` devolver o endpoint usado, para o diagnóstico aparecer na UI em vez de exigir `strings` no binário.
5. **Economia de ferramenta — build de release completo por mudança de versão.** Dois builds custaram ~5 min no total (o segundo incremental). Aceitável; nada a mudar.

## Lições

- **Fixture que não reproduz a condição real não guarda nada.** Se a guarda depende de um xattr, o fixture precisa do xattr.
- **Prova de update precisa de duas versões compiladas.** Bumpar só o `Info.plist` não serve: a versão que o app reporta vem do `tauri.conf.json` compilado (`PackageInfo`). O segundo build é o que torna a prova honesta.
- **Guardar o `$work` de um self-check vale mais que reler o log.**
- **O `open -n` do restart herda o ambiente**, então `OPENRIDR_HOME` isolado sobrevive à atualização — o que permite provar o update sem tocar no app home do usuário.
- **Um `pubkey` errado não reprova nada e quebra tudo.** A conferência contra a chave local e contra a cópia commitada é barata e cobre o modo de falha mais caro da frente.

## Correções da revisão independente

Preenchido depois do `/code-review` (rodada 1).

## Follow-ups

- **Canal real do GitHub não exercitado**: o passo `Publish the update channel` do `release.yml` só roda no próximo release por tag; até lá o upload e o download pelo app estão não verificados.
- **Artefato Developer ID + notarizado**: `verify_updater_artifact` (codesign/spctl/stapler no app extraído) só rodou com os fakes do self-check. O primeiro release real é a prova.
- **Botão de cancelar o download**: limite declarado da spec (fechar o app cancela). Exige estado de operação em voo no shell.
- **About OpenRidr com a versão**: hoje a única forma de ver a versão em execução é pedir uma checagem.
- **Proxy corporativo no updater**: o reqwest do plugin lê o System Settings, mas nada foi medido atrás de proxy.
- Não verificado: Windows, macOS Intel, Gatekeeper/quarentena no app atualizado, TCC depois do update, leitor de tela real.
