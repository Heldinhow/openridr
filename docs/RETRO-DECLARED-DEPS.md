# Retro — Dependências importadas passam a ser declaradas (#223)

Sessão filha do orquestrador, worktree `fix/issue-223-unlisted-deps`. Escopo entregue: ticket #236
(sub-issue de #223), spec `docs/SPEC-DECLARED-DEPS.md`, evidência `docs/evidence/declared-deps.md`.
Mudança de produto: três linhas em `package.json` e três no `bun.lock`. Nenhuma linha de `src/`.

## O que funcionou e deve continuar

- **Medir o mecanismo antes de confiar no critério de aceite.** O ticket pedia "`bun install
  --frozen-lockfile` falha antes e passa depois". O implementer mediu e o failing-first **não existe** no
  bun 1.3.9 (ver achado 1). A sessão reconferiu num diretório descartável antes de repetir a afirmação no
  PR. Sem essa segunda medição, o PR teria vendido um critério que não protege nada.
- **Grill curto e com recomendação em cada pergunta.** Cinco perguntas, uma rodada, todas respondidas de
  primeira: as decisões que sobraram (declarar vs. trocar import, faixa `^`, onde registrar a exceção,
  guarda de regressão, superfície de prova) eram exatamente as que mudariam o diff. Nenhuma pergunta de
  fato — os fatos (`codemirror` não reexporta, `url()` no CSS, versões no lock) foram medidos antes.
- **Um ticket, e o quiz foi pré-aprovado.** A mudança é uma transação (manifesto + lock + registro), sem
  fatia vertical possível. O orquestrador pré-aprovou o caso de um ticket e a sessão não gastou uma
  rodada.
- **O implementer subagent mediu em vez de obedecer.** Recebeu a ordem de provar o failing-first, não
  conseguiu, investigou o mecanismo do bun num diretório descartável e reportou a contradição com a
  evidência. Um subagente que reporta "não reproduziu, e aqui está o porquê" vale mais que um que
  inventa a falha.
- **Portas alternativas em vez do slot fixo.** `OPENRIDR_E2E_UI_PORT=1520 OPENRIDR_E2E_HOST_PORT=4417`
  (mecanismo da #214) rodou dois specs sem disputar o par 1420/4317 e sem pedir slot.

## Achados de ambiente (ordenados por custo)

1. **`bun install --frozen-lockfile` não é guarda de dependência não listada.** Medido no bun 1.3.9 (a
   versão pinada em `ci.yml`, `e2e-smoke.yml`, `release.yml` e `dependency-audit.yml`): com a tabela
   `workspaces."" dependencies` do lock desatualizada, o comando sai **0**, não falha e não reescreve o
   lock — a tabela fica desatualizada em silêncio. Ele só falha quando o **conjunto resolvido** mudaria, e
   os três pacotes já estavam em `packages` por transitividade de `codemirror`.
   *Melhoria*: ticket de dependência não deve usar "`--frozen-lockfile` passa" como critério de prova —
   o critério útil é o **diff do lock** (só a tabela do workspace raiz, nenhuma entrada nova em
   `packages`). Vale para o próximo ticket que declare dependência, e é o tipo de coisa que o `/to-tickets`
   pode herdar daqui.
2. **A guarda de verdade é o `knip`, e ela continua adiada.** Esta issue fechou três instâncias, não a
   classe: um import novo sem declaração volta a passar em silêncio, porque nada no CI compara import com
   `package.json`. A lacuna foi decisão do grill (a guarda é a adoção do `knip`, frente adiada de
   `docs/SPEC-CI-PRACTICES.md`), e fica registrada aqui como dívida declarada, não como cobertura.
   *Melhoria*: quando o `knip` for adotado, a config dele já nasce com
   `@fontsource-variable/{inter,jetbrains-mono}` em `ignoreDependencies` — o motivo está em
   `docs/decisions.md`.
3. **Worktree nova não commita antes de `bun install`.** O pre-commit roda `bun run check` e o `tsc` não
   existe sem `node_modules`: o primeiro commit desta sessão morreu em `tsc: command not found`. O
   `AGENTS.md` já avisa para rodar `bun install` antes de `check`, `build` e E2E, mas não menciona o
   commit — que é justamente o primeiro passo de quem segue o fluxo de tickets.
   *Melhoria*: acrescentar "ou de commitar (o hook roda `bun run check`)" naquela linha do `AGENTS.md`.
   Caminho de risco alto, então fica como proposta para um PR próprio, não para este.
4. **Nome de arquivo de spec não é prova de cobertura.** A spec desta frente dizia que
   `editor-find-count.spec.ts` exercitava o `gotoLine` do `@codemirror/search`; o review conferiu e o
   `gotoLine` é coberto por `medium-nav-navigation.spec.ts` (diálogo `.cm-goto-line`). O erro era de
   leitura de nome de arquivo, não de código.
   *Melhoria*: ao citar um spec como prova de um comportamento, `grep` nos títulos dos `test(...)` do
   arquivo antes de escrever a frase. Custou um achado de review e uma rodada de spec a mais.
5. **Evidência desproporcional.** O implementer entregou 151 linhas de evidência para um diff de 6 linhas
   (com o diff inteiro citado duas vezes). O conteúdo que importa — a medição do `--frozen-lockfile` — são
   ~20 linhas; o resto é transcrição de saída de comando.
   *Melhoria*: no prompt do implementer, pedir evidência com o comando e a linha de resultado, não a
   transcrição inteira; o diff inteiro já está no PR.

## O que ficou fora e por quê

- **Guarda de regressão** (teste que compara imports com `package.json`): decisão do grill. Seria um
  parser de import por regex para responder a mesma pergunta que o `knip` responde; entra com a adoção do
  `knip`.
- **Os outros achados do `knip`** (8 exports e 7 tipos sem uso): fora do escopo da #223, que é sobre
  declaração.
- **`@codemirror/autocomplete` em runtime**: nenhum spec cobre o autocomplete do editor, e esta issue não
  criou um. Antes e depois, cobertura nenhuma.
- **Windows e macOS Intel**: não executados, como no resto do repositório.
