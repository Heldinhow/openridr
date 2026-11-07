# Retro — diff de um arquivo de um commit pelo Git log (#101)

Sessão filha do orquestrador, worktree `feat/issue-101-git-log-file-diff`. Escopo entregue: spec `docs/SPEC-GIT-COMMIT-DIFF.md`, host `git.commitDiff` (#169), aba de diff do commit no Git log (#170), evidência `docs/evidence/git-commit-diff.md` (#171). Duas rodadas de revisão independente (r1 com 5 achados, re-check com 1 médio restante).

## O que funcionou e deve continuar

- **A lição do pathspec já existia no repo.** O #155 (`git mergeContinue`) já usava `git --literal-pathspecs add -u` com o comentário de que `a*.txt` casaria `a-other.txt`; os construtores de diff não usavam. Achar a mesma decisão em outro arquivo teria evitado o achado médio da revisão — vale procurar o padrão no módulo antes de escrever o comando, não só a API.
- **Medir o Git antes de escrever a spec.** O rename só pareia com os **dois** caminhos no pathspec; com só o novo, o Git devolve add+delete. Isso foi medido num repositório descartável antes da spec, virou decisão escrita e é a linha que o teste de host prova — em vez de ser descoberto na revisão.
- **O host como dono dos dados, de novo.** `git.commitDiff` devolve os dois lados como blob e o React só monta linhas com a função do #48. Nenhuma linha de diff em TypeScript, nenhuma dependência nova, e a evidência de host cobre rename/raiz/adicionado/deletado sem UI.
- **Extrair o tail antes de duplicar.** `assemble` saiu do `file_diff` para servir os dois métodos; os 54 testes antigos foram a rede de segurança da extração e nenhum mudou. Sem isso, o diff de commit seria uma segunda cópia de ~60 linhas com comportamento divergindo na primeira correção.
- **Falha-antes barata por mutação local.** Três mutações (base do rename, pathspec, `is_tracked`) rodaram em segundos e provaram que os testes mordem. A terceira mostrou que a flag em `is_tracked` **não** era load-bearing — e a evidência foi corrigida para dizer isso em vez de vender prova que não existe.
- **O re-check pegou a metade que faltava.** A correção do fechamento de aba tratou a aba inativa; fechar a **ativa** com a outra aberta continuava deixando a área vazia. Uma segunda passada com o mesmo modelo custou minutos e fechou o estado.

## Achados de ambiente (ordenados por custo)

1. **`bunx tauri build --debug` não roda numa worktree sem `src-tauri/resources`.** O build reprova em `resource path 'resources/csharp-ls' doesn't exist`. Copiar `csharp-ls/`, `dotnet/` e o binário do `netcoredbg` do app instalado resolveu em ~1 min, mas o par de análise do app sob teste passa a ser o da **alpha.9 instalada**, não o do checkout. Para uma feature de Git isso não muda nada; para uma que dependa da versão exata, é prova enganosa.
   *Melhoria:* o `scripts/dev-app.sh` que faltava (já pedido no `RETRO-GIT-DIFF`, item 1) **entrou na `main` durante esta sessão** (#175, `a5eda60`): ele sobe o app de dev com o par embutido e o workspace por argv, sem `tauri build`. Esta sessão ainda pagou o custo porque a branch nasceu antes; a próxima usa o script.
2. **O slot do app é disputado entre sessões, e o `pgrep` do orquestrador envelhece.** O orquestrador liberou o slot, mas quando fui abrir havia um `target/debug/openridr /tmp/issue91-dev/...` da sessão da #91. O `desktop.sh up` recusou (correto) e eu avisei em vez de matar. Sem custo além de um round-trip; a regra "nunca `pkill` por nome" já estava no lugar.
3. **A suíte completa do host é sensível a carga.** Com o bundle recém-construído e o app aberto, três testes de LSP falharam no run completo (`--test host`) e o mesmo binário passou 108/108 sozinho depois (284 s contra 104 s do run verde anterior). Registrado na tabela de `docs/evidence/test-stability.md`; não é código.
4. **O `%h` do Git não é sempre 7 caracteres.** O log cortava o hash em 7 e o painel usava `%h`; num repositório grande a aba e o painel mostrariam hashes curtos diferentes para o mesmo commit. O revisor pegou; agora os dois pedem `%h` ao Git.

## As duas rodadas de revisão

| Rodada | Achados | Correção/prova | Limite que ficou |
|---|---|---|---|
| r1 (`edbc696`) | 3 médios: caminho tratado como pathspec (`a[1].txt` também casava `a1.txt`); metadados do commit lidos sem limite; fechar aba inativa zerava a aba visível. 2 baixos: hash curto divergente entre log e painel; evidência atribuindo `too_large` ao teste do #48 | `--literal-pathspecs` nos três construtores de comando; `spawn_bounded` no `commit_ref`; fechar só limpa a seleção quando a aba fechada é a ativa; `git.log` pede `%h`; teste próprio de `too_large`. Falha-antes medida nas duas mutações do rename e na flag do commit | A flag em `is_tracked` não reprova nada neste fixture (o Git tenta o literal primeiro) — declarado como intenção, não como prova |
| re-check (`3ab480a`) | 1 médio: fechar a aba **ativa** com a outra aberta deixava o editor vazio ao lado de uma aba visível e não selecionada | fechar a ativa seleciona a outra quando existe; dois testes novos saíram das observações (tab no assunto do log, nome literal não é "não rastreado") | Correção de UI provada por leitura + `tsc`, sem reabrir o app |

## O que ficou fora e por quê

- **E2E**: a issue pedia um spec contra repositório descartável; a decisão do Helder na #67 (repetida no grill) é que Playwright é só smoke noturno e nunca verificação de UI de sessão. A prova é host + unidade + app real, e a troca está na spec e na evidência.
- **Navegação por seta entre as duas abas especiais**: exigiria `agent-desktop press --headed`, que move o foco do usuário. O caminho é o mesmo de antes com um índice a mais, coberto por `tsc` e leitura.
- **`too_large` e `unreadable` no diff de commit na tela**: o primeiro tem teste de host (blob acima de 1 MiB), o segundo segue ramo defensivo sem fixture.
- **Merge commits**: o log não lista arquivos de merge, então não há linha para clicar; o primeiro pai está documentado na spec.
- **Windows e macOS Intel**: não executados, como no resto do repo.
