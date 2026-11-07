# RETRO — A captura do E2E fora do repositório (issue #217, tickets #248–#249)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-E2E-EVIDENCE-OUTPUT](SPEC-E2E-EVIDENCE-OUTPUT.md).
Evidência: [evidence/e2e-evidence-output.md](evidence/e2e-evidence-output.md). Origem: o retro da #210,
onde a sujeira aconteceu duas vezes.

## O que se sustentou

- **O grill de fatos antes da spec mudou o escopo para melhor.** A issue nomeava três specs; a leitura
  encontrou **treze** com o default no repositório e **dois** que gravavam caminho literal e ignoravam
  a variável — esse segundo grupo não estava na issue e virou critério de aceite.
- **O seam existente absorveu a mudança sem módulo novo.** O módulo de ambiente do E2E já era o ponto
  por onde config, `globalSetup` e specs leem portas, home e URLs; ganhou dez linhas e os quinze specs
  ficaram com uma linha a menos cada.
- **A guarda em `tests/unit` foi a decisão certa.** É a única parte da prova que roda no CI
  (`bun run test:unit`), fecha a classe do bug — qualquer spec novo que nomeie o caminho versionado
  reprova — e custou ~40 linhas, com vermelho medido (15 ofensores) antes da migração.
- **O implementer subagent entregou com TDD de verdade:** mostrou a guarda vermelha e a verde, e
  declarou explicitamente o que não rodou (a prova de E2E) em vez de inventar.
- **O code review pegou duas coisas que passaram.** A guarda só olhava `.spec.ts` no nível de cima e só
  a forma literal do caminho; e os três specs opt-in continuavam lendo `process.env` direto, fora do
  seam. Os dois achados foram corrigidos e **verificados em runtime**: `workspace` 16/16, `welcome` 6/6,
  `ui-layout` a 1600 2/2, todos gravando no diretório apontado.
- **`--global-timeout` resolveu o "run que falha" sem inventar falha artificial:** o run aborta com
  exit 1 depois de já ter gravado as capturas, que é exatamente o cenário da issue.

## O que quebrou / custou

- **A porta 1720 é proibida pelo Chrome.** ~20 min até o erro aparecer (`net::ERR_UNSAFE_PORT`), e o
  harness não avisa: `port()` valida 1–65535 e nada mais. Host e Vite sobem normalmente, então o
  sintoma parece bug do app. 1719–1723 estão na lista do navegador.
- **Disco cheio no meio da sessão.** `ENOSPC` no `globalSetup` do E2E, com o container em 127 Mi livres.
  Custou uma parada e uma decisão do orquestrador. O `doctor.sh` do `verify-openridr` checa bun, cargo,
  `node_modules`, SDK, Chrome, portas e permissões — **não** checa espaço.
- **A premissa da issue estava meio errada, e só a medição mostrou.** `build.spec.ts` grava `11-*.png`,
  que **não** são versionados: o dano dele é arquivo não rastreado dentro do repo (o que faz
  `git worktree remove` recusar), não `git status` sujo. Quem reescreve PNG versionado é
  `csharp-ls`/`find-usages`/`zy-gutter`. A spec foi escrita antes dessa medição.
- **O `.gitignore` mente sobre o diretório.** `docs/evidence/screenshots/` está lá e, ainda assim, 86
  arquivos são rastreados. Levou um `git ls-files` para descobrir; a primeira leitura ("é ignorado,
  logo não suja nada") estava errada.
- **O primeiro commit foi abortado pela limpeza de disco** e só não se perdeu porque conferi
  `git status` depois: o `commit-only.sh` não deixa rastro quando o processo morre no hook.

## Lições

- **Diretório de evidência dentro do repositório é dívida.** O `.gitignore` não desfaz o que já está
  versionado, e qualquer default que aponte para lá herda o risco.
- **Porta de sessão precisa de validação do harness, não de memória humana.** A lista do Chrome é
  estável e pequena; quem escolhe a porta é um agente.
- **Guarda de higiene precisa de um caso que prove a rejeição.** Sem isso ela passa por acidente — foi
  o que o review mostrou (varredura não recursiva, forma segmentada do caminho).

## Follow-ups (propostos, não feitos nesta issue)

1. **`port()` recusa porta da lista do Chrome** (1719–1723, 6000, 6665–6669, 10080) com mensagem
   nomeando a porta — check determinístico no harness, ~5 linhas mais teste.
2. **`doctor.sh` avisa sobre espaço livre** abaixo de um limite (o E2E cria ~200 Mi por home de porta).
3. **`.gitignore`: comentário na linha de `docs/evidence/screenshots/`** dizendo que os 86 PNGs
   continuam versionados e que a linha só vale para arquivo novo.
4. **`verify-openridr`: uma linha sobre `--global-timeout`** como forma de produzir um run que falha
   depois de gravar, sem esperar uma falha real.
5. **`code-review`: guarda/teste de higiene precisa de um caso que demonstre a rejeição** — é a regra
   que teria pegado os dois achados do review antes de mim.
