# RETRO — conflito de disco determinístico no E2E (issue #273, ticket #278)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-E2E-DISK-CONFLICT](SPEC-E2E-DISK-CONFLICT.md).
Evidência: [evidence/e2e-disk-conflict.md](evidence/e2e-disk-conflict.md). Origem: `workspace.spec.ts:326`
falhou no run 42412536089 da #245 e passou 28/28 no mesmo SHA local.

## O que se sustentou

- **Espiar antes de prometer.** O seam (`page.route` segurando o `file.read` do arquivo ativo) foi medido
  antes da spec: falha-antes 5/5 com um tick de poll forçado, 20/20 verde sem ele. A spec descreveu um
  mecanismo provado, não uma hipótese — e o spike achou o `Route is already handled!` que teria custado
  uma sessão depois.
- **Commitar o spike e entregar o ponteiro ao implementer subagent.** O subagente não re-derivou nada:
  partiu de `bfb841f`, provou vermelho-verde na worktree dele, escreveu a evidência e o merge foi
  fast-forward. Um ticket pequeno deixou de ser motivo para furar a regra do subagente.
- **O teste que prova o poll já existia.** "reloads a clean buffer…" espera o banner com timeout; por isso
  congelar o poll neste teste não tirou a cobertura do caminho periódico.
- **`untilHeld` transforma o gate em prova.** Sem esperar o poll bater na leitura segurada, o teste
  passaria com o gate removido (~7% de falha por rodada, medido: 20/20 verde sem ajuda). Com a espera,
  remover o gate derruba o teste sempre.
- **Code review sem achado alto/médio.** O único achado baixo era o link da retro no índice antes de a
  retro existir — fechado neste arquivo.

## O que quebrou / custou

- **A corrida não reproduz localmente.** 20/20 verde sem ajuda: a janela entre a escrita e o clique é
  ~100 ms contra um tick de 1500 ms. "Provar que consertou" teria sido teatro; o que valeu foi **forçar o
  poll a perder** (um `waitForTimeout(1600)` entre a escrita e o clique) e ver 5/5 vermelho no assert do
  conteúdo em disco.
- **A falha tem dois sintomas.** No runner estourou `toBeVisible` do banner; localmente o banner seguiu
  visível e quem estourou foi o conteúdo em disco. `click()` resolve quando o evento é despachado, não
  quando o `file.write` volta — então um assert de visibilidade pode passar com a escrita ainda em voo.
- **Copiei meio helper.** O `page.unroute` chamado com o handler parado no gate faz o `route.continue()`
  estourar `Route is already handled!`; o `delayNextTrustGrant` de `gates.spec.ts` já tinha o
  `finished`/`finally` que evita isso. Copiar o helper inteiro, não a metade.
- **Resolvi o conflito append-only ao contrário.** `docs/README.md` diz, no fim da tabela, que fica **a
  entrada desta branch primeiro**; resolvi com as da `main` na frente e tive de refazer nos três arquivos.
  A regra estava escrita — o custo foi não ler até o fim antes de resolver.
- **O teste ficou 1,7 s mais lento** (2,5 s → 4,2 s): o `untilHeld` espera até um tick. Barato no smoke,
  mas é o preço de o gate não poder ser removido em silêncio.

## Lições

- **Para provar que uma corrida foi consertada, primeiro faça-a perder sempre.** Um delay que garante o
  entrelaçamento vale mais que N rodadas de sorte; sem ele o número "20/20 verde" não distingue conserto de
  coincidência.
- **Seam armado não é seam exercitado.** Um gate que o teste instala mas nunca espera ser usado pode ser
  removido sem ninguém notar; espere a condição (`untilHeld`) para o gate virar prova.
- **`click()` não espera o efeito assíncrono do handler.** Asserção de visibilidade logo depois do clique
  pode passar com a escrita ainda em voo; o assert que fecha a conta é o do estado final (disco, log).
- **Helper de gate: copie o desenho de limpeza junto** (gate → `finished` → `unroute`), nunca só o gate.
- **Docs append-only: leia a regra de resolução antes de resolver o conflito.** Ela está no fim da tabela
  de `docs/README.md`, e a ordem é "esta branch primeiro".

## Follow-ups

- **Guarda para os três arquivos append-only** (`GLOSSARY.md`, `docs/README.md`, `docs/decisions.md`): um
  check de CI que reprove um merge que **remove ou reordena** linha antiga neles. É `.github/workflows/**`
  (`risk:high`), fora desta frente.
- **`docs/evidence/test-stability.md` ganhou a linha com os dois números** (20/20 sem ajuda e 5/5 forçado).
  Se a tabela crescer, vale uma coluna "reprodução forçada" separando isso de "não reproduzido".

## Não verificado

- Runner do GitHub: a corrida foi medida localmente com um tick forçado, não no CI. O `workflow_dispatch`
  do smoke não foi disparado (o grupo de concorrência está em uso pela #251).
- `run.spec.ts:352` / #251: outra família (build falhou antes), fora do escopo.
- Windows e macOS Intel.
