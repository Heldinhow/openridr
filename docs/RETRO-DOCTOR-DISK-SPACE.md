# RETRO — espaço livre no `doctor.sh` (issue #259, ticket #267)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-DOCTOR-DISK-SPACE](SPEC-DOCTOR-DISK-SPACE.md).
Evidência: [evidence/doctor-disk-space.md](evidence/doctor-disk-space.md). Origem: o retro da #217, onde o
`ENOSPC` custou uma rodada de E2E e só apareceu como cópia de arquivo falhando.

## O que se sustentou

- **O grill de fatos antes da spec mudou os números.** A issue propunha "~500 Mi é o suficiente"; a
  medição mostrou que o piso pertence à política de concorrência do orquestrador (até quatro sessões em
  paralelo, ~200 MiB de app home cada, mais caches), e os pisos viraram 3072/1024 MiB. Sem perguntar, o
  check teria nascido com um número que não incomoda ninguém — e que também não avisa ninguém.
- **O seam já existia.** O `df` resolvido pelo `PATH` deixou o teste provar as três severidades sem
  encher disco e sem variável só-de-teste no script. O `doctor.sh` ganhou 14 linhas e nenhum caminho
  novo de configuração.
- **O implementer subagent pegou um bug no próprio caminho.** A primeira versão fazia
  `awk 'int($4/1024)'`, que transforma saída não numérica em `0` e imprimia `FAIL disk 0 MiB` — um
  número inventado. O caso "`df` sem número" existe porque o teste foi escrito antes: hoje a linha diz
  `unknown`, nunca `0`.
- **O code review pegou o teste passando pelo motivo errado.** O `df` falso respondia a qualquer
  invocação, então trocar o volume medido ou tirar o `-Pk` continuava verde. O stub agora recusa
  chamada fora de `df -Pk <diretório medido>`; medido: `df -k` no lugar de `df -Pk` → 3 casos vermelhos.
- **A falha-antes foi medida duas vezes**, pelo subagente e pela sessão: com o `doctor.sh` de `5779634`
  são 4 falhas, com o de `HEAD` nenhuma.

## O que quebrou / custou

- **O `df` do BSD muda o layout sem `-P`.** `df -m` no macOS acrescenta `iused`/`ifree`/`%iused`, então
  a coluna "Available" não é a mesma de um `df -k`. Foi preciso escolher `-Pk` (POSIX) e não `-m`.
- **O disco desta máquina não é um volume só.** O worktree está num SSD externo e o `$TMPDIR` no disco
  interno: medir "o disco" sem dizer qual volume mede daria `ok` no volume errado. A linha imprime o
  diretório medido por isso.
- **Nada obriga a sessão a rodar o doctor.** O check novo só ajuda quem o executa; o
  [RETRO-IN-APP-UPDATER](RETRO-IN-APP-UPDATER.md) já tinha registrado a mesma lacuna para a superfície C.
- **O teste novo não roda no CI.** `tests/shell/` é rodado à mão; a suíte de shell do repositório não
  tem job.

## Candidatos para o ambiente (por severidade)

1. **O harness do E2E recusar a rodada abaixo do piso duro.** Hoje o `doctor.sh` avisa, mas o
   `globalSetup` do Playwright continua tentando e morre no meio de um download. Um check de espaço no
   `globalSetup` (ou no `bun run test:e2e`) fecha a classe em vez de depender de alguém ler a saída do
   doctor. Issue própria: mexe no harness, não no doctor.
2. **Rodar `tests/shell/*.test.sh` num job de CI.** O repositório tem oito testes de shell, incluindo
   este, e nenhum job os executa; o `governance` só roda `actionlint`/`zizmor`. Um job `dash` sobre
   `tests/shell/` transformaria "rodei à mão" em guardrail. Custo: minutos de CI, risco de flake baixo
   (os testes usam stubs).
3. **O doctor virar pré-requisito declarado do E2E.** Uma linha no `SKILL.md`/`AGENTS.md` dizendo que a
   prova de UI sem doctor é inválida — mais barato que (1), mas continua dependendo de obediência.
4. **O vazamento que encheu o disco.** As ~4.600 pastas `.tmp*` (~18 GB) são testes do host que não
   limpam o `TempDir`. Nenhum check de espaço conserta isso; é issue própria (e o doctor é somente
   leitura por decisão).

## Lições

- **Um check que imprime número precisa distinguir "medido" de "desconhecido".** `0` é um valor válido e
  mentiroso; `unknown` é a resposta honesta para uma medição que falhou.
- **Um dublê de processo deve afirmar a forma da chamada.** Responder a qualquer argv faz o teste passar
  quando o código mede o volume errado.
- **Piso de disco é política, não estimativa.** O número certo vem de quem sabe quantas sessões rodam em
  paralelo; a issue só sabia que 127 Mi era pouco.

## Não verificado

- Superfícies A/B/C do `verify-openridr`: nenhuma interface, host ou app mudou.
- Disco realmente cheio: `note`/`FAIL` provados com `df` falso; encher o volume interno não é aceitável
  nesta máquina.
- Windows e macOS Intel, como no resto do repositório.
