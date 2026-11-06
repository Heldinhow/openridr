# SPEC — Git remote acceptance (issue #50)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe: [#50](https://github.com/Heldinhow/openridr/issues/50).

## Problema

O painel Git foi endurecido para o **Commit** e para o **working-tree diff** (#36, #48), mas `push`, `pull`, troca de branch e histórico nunca foram exercitados de ponta a ponta no app Tauri real, contra um repositório com remoto. Ninguém sabe hoje se um commit novo chega ao remoto, se o pull de uma mudança recebida incorpora o trabalho da contraparte, se a troca de branch confirmada na UI corresponde ao `HEAD` em disco, nem se o histórico listado bate com o `git log`.

Sem um protocolo reproduzível, cada rodada improvisa um remoto, improvisa os cenários e deixa como prova apenas a narrativa de quem rodou.

## Solução

Um **protocolo de aceitação** versionado, com **remote de teste local** reproduzível (bare + dois clones descartáveis) e evidência por cenário no app Tauri real. Cada cenário prova o efeito no disco e no remoto além da tela: `rev-parse`, `log` no bare, bytes do arquivo, marcador de operação em andamento. Bug encontrado vira issue própria; esta frente não corrige produto.

O protocolo vale para qualquer rodada futura de Git: o fixture é um script, não um diretório montado à mão.

## User stories

1. Como pessoa que mantém o OpenRidr, quero um remoto de teste que não publique código nem dependa de login, para exercitar Git real sem tocar em projeto privado ou remoto externo.
2. Como pessoa que mantém o OpenRidr, quero montar esse remoto com um comando, para repetir a aceitação sem improvisar diretórios.
3. Como pessoa que valida a release, quero que o workspace aberto no app seja a raiz do repositório, para que os caminhos do painel Git sejam os do repositório e não os de um subdiretório.
4. Como pessoa que valida a release, quero abrir esse workspace pelo mesmo caminho do Finder/CLI (bundle real, `OPENRIDR_HOME` isolado), para que a prova seja do app e não do navegador.
5. Como pessoa que valida a release, quero que o protocolo diga qual commit/versão está sob teste, para não atribuir a prova ao `main` errado.
6. Como pessoa que valida a release, quero fazer um commit pelo painel e vê-lo no remoto, para saber que o caminho `commit → push` fecha.
7. Como pessoa que valida a release, quero ver o diálogo de Push com remoto, branch, upstream e commits à frente antes de confirmar, para que o push nunca seja cego.
8. Como pessoa que valida a release, quero que a rejeição de push por non-fast-forward apareça na tela e preserve o commit local, para não perder trabalho quando a contraparte publicou primeiro.
9. Como pessoa que valida a release, quero incorporar por **Update…** um commit publicado pela contraparte, para saber que o pull traz conteúdo de verdade.
10. Como pessoa que valida a release, quero distinguir **fetch** de **pull** no protocolo, para explicar de onde vem o contador `↓` e por que ele pode estar defasado antes do pull.
11. Como pessoa que valida a release, quero que o pull divergente recuse sem perder os dois lados, para provar que a operação é segura quando o merge não é fast-forward.
12. Como pessoa que valida a release, quero ver a árvore suja bloquear Update e troca de branch com motivo explicado, para saber que o app não auto-stasha.
13. Como pessoa que valida a release, quero criar uma branch e voltar para a anterior, com `rev-parse` e arquivos conferidos, para saber que a troca de branch é real.
14. Como pessoa que valida a release, quero fazer checkout de uma branch remota publicada pela contraparte, para saber que o app cria a branch de tracking em vez de um `HEAD` solto.
15. Como pessoa que valida a release, quero comparar o histórico do painel com `git log`, incluindo arquivos do commit selecionado e filtro, para saber que o histórico não inventa nem omite revisão.
16. Como pessoa que valida a release, quero que a evidência registre ação, estado e comando/resultado de cada cenário, para que a prova seja auditável sem a sessão.
17. Como pessoa que mantém o OpenRidr, quero que o que não rodou (autenticação, rede, Windows, Intel) apareça como não verificado, para não vender suporte que não existe.
18. Como pessoa que mantém o OpenRidr, quero que cada bug reproduzido vire issue com reprodução e prova, para que a correção comece de um ticket e não desta sessão.

## Implementation decisions

- **Escopo é aceitação, não feature.** Esta frente entrega script de fixture, override mínimo do lançador de verificação, protocolo, evidência, retro e decisão. Correção de produto acontece em ticket próprio.
- **Remote de teste local é bare em `$TMPDIR`, com dois clones descartáveis.** `origin.git` (bare) é o remoto; `user` é o clone que o app abre; `peer` é a contraparte, dirigida só por shell. Nada de push/pull/commit em projeto do Helder ou em remoto externo.
- **O workspace é a raiz do clone.** O conteúdo de `fixtures/OrderDesk` é copiado para a raiz do clone `user`, de modo que o repositório e o workspace coincidam; o app descobre o repositório por `rev-parse --show-toplevel`, então isso também prova o caminho de subdiretório, não o substitui.
- **Configuração de git só nos repositórios descartáveis.** `user.name`/`user.email` e qualquer ajuste vivem no `git config` local dos clones; nada global.
- **O fixture nunca apaga nada.** Sem argumento ele cria um state novo e privado com `mktemp` sob a raiz temporária real; um argumento precisa ser um caminho absoluto **novo** dentro dessa raiz. Caminho existente (diretório, arquivo ou symlink pendente), `/`, home, o checkout, caminho relativo, `..` ou symlink apontando para fora são **recusados** sem apagar nem reutilizar nada. Recomeçar é rodar de novo sem argumento; o state anterior fica.
- **O lançador de verificação ganha um override, não um modo novo.** `desktop.sh up` passa a aceitar um workspace preparado (variável de ambiente) e, nesse caso, não copia a fixture; o default continua sendo `fixtures/OrderDesk`. As checagens de instância única e o PID gravado continuam as mesmas.
- **Fetch e pull são coisas diferentes e o protocolo diz isso.** O contador `↓` do painel é a distância entre `HEAD` e a ref de tracking *local*; ele só reflete a contraparte depois de um `fetch` (manual, no clone descartável) ou depois de um `push` próprio. O `Update…` roda `git pull` (fetch + merge) e por isso incorpora mesmo com `↓ 0` na tela. A evidência registra as duas leituras.
- **Conflito é cenário de recusa, não de resolução.** A UI não oferece abort nem commit de merge; o protocolo prova que o pull divergente falha, que os dois commits sobrevivem e que o estado de operação em andamento aparece. Concluir ou abortar pela UI, se faltar, é lacuna registrada em ticket, não correção desta frente.
- **Cenário destrutivo por último e em branch descartável.** O conflito roda em branch própria, depois dos cenários de caminho feliz. Proibido `reset --hard`, `clean`, `push --force` ou apagar trabalho fora dos diretórios efêmeros de teste.
- **Bug reproduzido vira issue** com reprodução, comando/resultado e captura quando houver, com categoria (`bug`/`enhancement`) e o papel de triagem da convenção do repo.

## Testing decisions

- **Um teste bom aqui prova o efeito, não o relato.** Cada cenário tem um comando de conferência fora da UI (`git rev-parse`, `git log --oneline` no bare, `cat`/`git diff` do arquivo, presença de `.git/MERGE_HEAD`) e a captura da tela correspondente.
- **A prova é do app Tauri real**, dirigido por `agent-desktop` com `OPENRIDR_HOME` isolado e o `.sln` do clone em argv; navegador e Playwright de sessão ficam fora. Uma instância por vez, fechada pelo PID gravado.
- **Cenários (S1–S10)**: status inicial; commit→push; push rejeitado por non-fast-forward; pull fast-forward; pull divergente em branch descartável; árvore suja (Update/checkout bloqueados, push permitido); branch nova e volta; checkout de branch remota com tracking; histórico contra `git log`; branch sem upstream.
- **Prior art**: `docs/evidence/git-diff.md` (mesmo formato ação → estado → comando/resultado) e `docs/evidence/03-open-with.md` (app real com argv e `OPENRIDR_HOME` isolado).
- **O fixture e o launcher têm teste de shell próprio**: `sh tests/shell/git-remote-fixture.test.sh` cobre as recusas que não podem apagar (existente com sentinela, `/`, `$HOME`, checkout, relativo/`..`, symlink para fora), o state novo íntegro (bare + `user` + `peer`, baseline único), dois defaults gerando states distintos, e o override inválido do launcher falhando **antes** de rotacionar o state anterior.
- **O que não rodou é declarado**: autenticação, transporte de rede, `force`, tags, Windows e macOS Intel entram como não verificados, com motivo.

## Out of scope

- Autenticação (credential helper, SSH, token) e transporte de rede (HTTPS/SSH): o remote de teste é local.
- `push --force`, tags, submodules, `git fetch` como recurso de produto.
- Amend, "commit and push" combinado e o diff de um commit do histórico (#101).
- Estado "sem remoto" em workspace próprio; a cobertura de upstream ausente usa uma branch local.
- Windows e macOS Intel.
- Qualquer correção de produto descoberta durante a aceitação.

## Further notes

- A rodada de referência usa o bundle de debug construído do commit sob teste; a versão do bundle é registrada junto com as dos artefatos de build. Pin de `netcoredbg`, runtime .NET 10 e `csharp-ls` não muda.
- Evidência em `docs/evidence/git-remote-acceptance.md`, capturas em `docs/evidence/git-remote-acceptance/`.
- Decisão registrada em `docs/decisions.md`; termos novos no `GLOSSARY.md`.
