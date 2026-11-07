# RETRO — porta restrita do Chrome recusada no load do E2E (issue #258, ticket #261)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-E2E-RESTRICTED-PORTS](SPEC-E2E-RESTRICTED-PORTS.md).
Evidência: [evidence/e2e-restricted-ports.md](evidence/e2e-restricted-ports.md). Origem: a sessão da #217
perdeu a rodada com o par 1720/4617 e morreu em `net::ERR_UNSAFE_PORT` no primeiro `page.goto`.

## O que se sustentou

- **O grill de fatos antes da spec mudou o teste, não o código.** A issue dizia "1719–1723"; a
  `kRestrictedPorts` do Chromium (`net/base/port_util.cc`, lida na fonte) tem 1719, 1720 e 1723 e aceita
  1721/1722. O teste pina 1721/1722/1725 como aceitas justamente para provar que a lista foi copiada em vez
  de inventada — e a correção foi comentada na #258, não escrita no corpo dela.
- **`playwright test --list` é a prova barata de falha no load.** 0,3 s, sem subir host, Vite ou navegador:
  o critério "o erro aparece no começo do run" ficou medido, não prometido. Um harness que valida no load
  não precisa de porta para ser provado.
- **Olhar os consumidores antes de fechar o escopo pagou.** `grep 'hostPort +'` mostrou os sete specs que
  sobem host próprio em `hostPort + 1..+7`; sem isso o ticket entregaria a recusa das duas variáveis e
  deixaria `hostPort=1716` quebrar no `+3`, no meio do run, exatamente como a issue descreve.
- **O implementer subagent entregou o ticket inteiro sem gastar o contexto da sessão** (2 commits, teste
  falhando antes e passando depois, 123 testes verdes). O merge foi fast-forward, sem conflito.
- **A evidência com par legal (2025/4917, home própria) provou que o harness continua subindo**:
  `environment.spec.ts` → 3 passed em 30,1 s.

## O que quebrou / custou

- **O teste unitário herdava as portas do ambiente.** Com `OPENRIDR_E2E_UI_PORT=2025` exportado — que é
  exatamente o que o fluxo de verificação deste repositório faz —, o teste da faixa derivada falhava porque
  a mensagem dizia `2025/1716` e o teste esperava `1420/1716`. Achado médio do code review, corrigido em
  `2adaeb6`: o subprocesso fixa o par antes de mesclar o ambiente.
- **O primeiro commit morreu no hook de pre-commit** (`tsc: command not found`) porque a worktree nova não
  tem `node_modules`. O `AGENTS.md` já avisa; custou um `bun install` de 2 s antes de o commit passar.
- **Medi `exit=$?` depois de um pipe e imprimi `exit=0` para um comando que falhou.** O `$?` era do `tail`.
  Refeito sem pipe: exit 1 em 0,3 s. O número que vai para a evidência tem que vir de um comando cujo código
  de saída não foi comido pelo paginador.
- **O índice `docs/README.md` ganhou o link da retro antes de a retro existir** (achado baixo do review).
  Aqui o link fecha no mesmo PR; numa frente que não escreve retro, ele fica pendurado.

## Lições

- **Lista copiada de fonte externa: confira a fonte e pine no teste o que ficou de fora.** As portas que a
  lista *não* tem (1721/1722/1725) são a prova de que ela foi copiada, não arredondada para uma faixa.
- **Módulo que valida no load se prova com `--list`**, sem porta, host ou navegador.
- **Teste que roda o módulo em subprocesso precisa fixar todo o ambiente que o módulo lê.** Este repositório
  exporta `OPENRIDR_E2E_*` no shell antes de rodar E2E; herdar isso é o caminho mais curto para um vermelho
  que só aparece na máquina de quem acabou de rodar a suíte.
- **Exit code de comando com pipe**: capturar sem pipe, ou `set -o pipefail`.

## Follow-ups

- **Rodar `bun test tests/unit` uma vez com um par de portas exportado no CI** (job `typescript`), porque é
  assim que a sessão roda no dia a dia e foi exatamente o que o review pegou. É mudança de CI (`risk:high`),
  fora desta frente.
- **O hook de pre-commit não diz o que fazer quando `tsc` não existe.** Candidato: mensagem "rode
  `bun install`" no `scripts/commit-only.sh` ou no hook. Hoje o erro vem do shell.

## Não verificado

- Suíte E2E completa (312 testes), smoke noturno, app Tauri real, Windows e macOS Intel.
- Nenhuma porta restrita foi exercitada de fato contra o Chrome: a prova é a recusa no load, não o
  `ERR_UNSAFE_PORT` real.
