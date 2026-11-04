# RETRO — correções do E2E alpha.5

Onda: 2026-11-04, branch `fix/e2e-alpha5`. Fonte: `docs/evidence/e2e-alpha5/REPORT.md` (20 achados) + decisões do Helder sobre SDK/toolchain.

## Números

- 15 tickets, 15 commits de implementação + 1 de spec/tickets + correções de revisão e documentos.
- 20/20 achados do relatório tratados; 18 corrigidos no código/app, 1 corrigido no script/relatório (BUG-007), 1 corrigido em texto (BUG-017).
- Camada SDK entregue: runtime 10 separado do SDK do projeto, descoberta ampla, mensagem por projeto, links vivos, csharp-ls com progresso/retry.
- Testes: `cargo test -p openridr-host --lib` 213/213; suítes de host e e2e tocadas verdes; app Tauri real dirigido por agent-desktop para P0/P1 e os P2/P3 principais.
- Limitações declaradas: Dock/AppleScript quit, capturas (Screen Recording negado), progresso visual do csharp-ls no app, embutir o netcoredbg (só proposta).

## O que funcionou

1. **Explorar a causa raiz antes de escrever ticket.** As notas por área (`A` saída/editor, `B` debug, `C` abertura/a11y/árvore, `D` skill, `E` SDK) deram file:line e evitaram 15 agentes redescobrindo o mesmo código. O BUG-002 virou uma linha; o BUG-001 virou um menu + um evento, não um rewrite.
2. **Um ticket = um commit = uma prova.** O histórico ficou bissexto: cada correção tem teste e evidência próprios, e a revisão achou dois problemas em commits isolados.
3. **A prova no app real pagou o custo.** Ela pegou o caminho `.staging` no registro do adaptador e o delete do nó de projeto quebrando o `.sln` — nenhum dos dois aparecia nos testes de host/e2e.
4. **O relatório estava errado em um achado.** BUG-007 não reproduzia no HEAD: o teste abriu a alpha.4 instalada porque `scripts/openridr` não dizia qual bundle abria. O script agora imprime o bundle e a versão; o doctor idem.
5. **Separar runtime de SDK foi a decisão certa.** O app real mostrou `Analysis runtime 10.0.14 · Project SDK 8.0.408` e o OrderDesk buildou/rodou/depurou sem pedir 10.

## O que doeu

1. **Permissão TCC não é estável entre sessões.** Accessibility caiu no meio e Screen Recording nunca voltou; o primeiro passe de verificação ficou sem capturas e precisou ser refeito por snapshot. Registre a limitação antes de prometer screenshot.
2. **Porta 1420 órfã.** A sessão do OpenChamber reiniciou e matou o Playwright, mas o Vite ficou; a suíte seguinte falhou em 1 s. Antes de rodar e2e, confira `lsof` das portas e mate só o processo do próprio repo.
3. **Menu padrão do Tauri é posicional.** Trocar o Quit assumindo "último item do submenu do app" funciona na versão atual, mas quebra em silêncio numa atualização. O `expect` no boot é a rede de segurança; um teste de app real continua necessário após subir o Tauri.
4. **Single-instance tem escopo de bundle id.** O lock do plugin é `/tmp/<identifier>_si.sock`; duas cópias (alpha.4 e alpha.5) não rodam juntas, mesmo com `OPENRIDR_HOME` diferente. Isso muda o workflow de comparar versões.
5. **O relatório tratou comportamento como bug.** BUG-008 (download sem diálogo) era decisão de produto; o Helder manteve o clique como consentimento e pediu transparência (checksum, progresso, retry, registro). BUG-019 era lacuna, não regressão. Separar "bug", "decisão" e "lacuna" no começo teria economizado uma pergunta.

## Lições para a próxima onda

- Antes de aceitar um repro, **confirme a versão do bundle que o script abriu**. Um `CFBundleShortVersionString` impresso no launch evita um achado falso inteiro.
- **Rode a prova no app real antes de escrever "corrigido"** no relatório; host/e2e não veem caminho cosmético nem consequência no `.sln`.
- **Delete/rename de nó de projeto precisa de decisão própria** (atualizar `.sln` ou proibir). A versão mínima proíbe; não ofereça a ação sem o backend que a torna segura.
- **Ferramenta baixada é estado de UI**: mostre origem, versão, checksum e o caminho final. O caminho de staging não é o instalado.
- **Permissão de tela é pré-requisito de evidência**, não detalhe: peça antes e registre "sem captura" se não vier.
- **Não empacote decisão de produto em ticket de bug.** O BUG-008 virou um requisito de transparência, não um diálogo novo.

## Pendências do Helder

1. **Embutir o netcoredbg**: proposta medida em `docs/PROPOSAL-NETCOREDBG-EMBED.md` (3.39 MB compactado, 9.14 MiB extraído, DMG ~8.6 MiB). Falta o OK; nenhum código foi feito.
2. **Dock Quit / AppleScript quit**: se precisar cobrir, exige `applicationShouldTerminate:` via objc/unsafe. Hoje é limitação declarada.
3. **Screen Recording**: sem ela não há capturas novas da superfície C; a prova atual é snapshot + disco.
4. **Duas cópias do app**: o lock de instância única é por bundle id; comparar alpha.4 e alpha.5 ao mesmo tempo deixa de funcionar.
