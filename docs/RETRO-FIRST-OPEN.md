# RETRO — primeira abertura com quarentena (Gatekeeper)

Onda: 2026-11-06, branch `fix/issue-45-gatekeeper-first-open` (base `46e7bc7`, alpha.11). Issue:
#45; spec: `docs/SPEC-FIRST-OPEN.md`; tickets: #71, #72; evidência:
`docs/evidence/first-open.md`.

## Números

- 6 commits (spec; harness+evidência; docs; dois de correção da revisão; diálogo/kill), 0 linha de
  código de produto. 8 arquivos tocados.
- Medições: 13 rodadas de abertura (E0/E1/E3/E3m/E3c/E4), 1 sonda de foco do diálogo, mais 2
  rodadas soltas durante validação (ver "O que atrapalhou").
- Resultado: diálogo do Gatekeeper em 0,40–0,53 s; clique→código 0,14–0,24 s; clique→janela
  0,97–1,42 s. Abrindo de dentro da imagem: clique→janela 5,1–6,6 s.
- 2 revisões delegadas (padrões + spec, depois re-check): 3 críticos e ~18 avisos, todos
  endereçados.

## O que funcionou

1. **Piloto antes do grid.** Uma rodada no alpha.9 instalado, cronometrando o clique de propósito,
   derrubou a hipótese da issue ("Gatekeeper verificando 18 Mach-O") em 5 minutos: o diálogo
   aparecia em 0,55 s e os 18 s eram a minha própria demora para clicar. Sem esse piloto o spec
   teria virado um plano de otimizar assinatura.
2. **Marcos, não total.** Separar `exec`/`prompt`/`click`/`code`/`window` transforma "demora 18 s"
   em três números com donos diferentes (sistema, humano, I/O da imagem). A conclusão "não é o
   bundle" sai do teto medido (≤ 0,53 s), não de opinião.
3. **Log unificado como cronômetro.** `present code-evaluation prompt` e `launchInQuarantine ==
   true` no log do `CoreServicesUIAgent` provam o mecanismo (processo suspenso, não verificação) e
   dão timestamps melhores que polling.
4. **Remover o ticket grampeado custou zero.** `Contents/CodeResources` está fora do selo: apagar
   mantém `codesign --verify --deep --strict` válido e mede o caminho online sem notarização nova —
   o que respeitou a decisão de não gastar submissão ao notarytool.
5. **Regra de corte escrita antes de medir.** "Variantes de bundle só se algum marco passar de
   ~2 s" evitou horas de variantes inúteis — e o texto final precisou ser reescopado para a
   condição certa (app no disco), o que a revisão pegou.

## O que atrapalhou

1. **Duas aberturas de GUI durante validação de argumentos.** Rodei `measure-first-open.sh` com
   argumentos que eu achava inválidos (um app de teste, um caminho que eu supunha ad-hoc) e o
   script executou de verdade, abrindo o app na máquina do Helder enquanto ele trabalhava. Numa
   delas a rodada ficou esperando clique e ele (ou quem estava na máquina) respondeu; na outra o
   `timeout` do meu shell matou o processo e **deixou o diálogo na tela** — que eu só vi na
   captura seguinte e fechei em *Cancelar*.
   Lição: script que toca GUI precisa de um modo que não abre nada e que seja o caminho natural de
   validação. Adicionado `--check-only`; os testes de argumento agora rodam com ele.
2. **O comentário "matar o app fecha o diálogo" estava errado.** O alerta é do
   `CoreServicesUIAgent`: quando o app nunca iniciou, matar o processo não fecha nada. O script
   passou a mandar `Escape` (modo `--click`) ou a avisar o operador antes da rodada seguinte.
3. **Primeira versão do harness não media o que dizia.** `if [ -z "$t_exec" ] && pid=$(pgrep … |
   head -n1)` mascarava o status da pipeline: `pid` virava vazio e `exec`/`code` nunca eram
   medidos, sem erro nenhum. Pegue por revisão delegada, não por teste — o script não tem teste.
4. **Números inconsistentes entre documentos.** Teto de 0,52 vs 0,53, "menos de meio segundo" com
   0,52, `0,24` vs `0,23`, e um excerto de log que misturava três rodadas. Só apareceram porque a
   segunda revisão comparou arquivo por arquivo. A tabela agora é uma linha por rodada, com deltas
   calculados dos valores brutos.
5. **Cópias de DMG herdam quarentena.** Uma condição "sem quarentena" saiu contaminada
   (`00c3;00000000;;<uuid do DMG>`) porque `ditto` de dentro de um DMG montado propaga o xattr —
   descoberto ao conferir o xattr em vez de confiar no nome da condição.

## O que fazer diferente

1. Toda condição medida ganha uma verificação do **estado da condição** antes da rodada (`xattr
   -p`, path do processo, cdhash) impressa na evidência; nunca inferir do nome do arquivo.
2. Script com efeito externo nasce com `--check-only` e com `--dry-run` para o que não for
   validável sem executar; validação de argumento não pode ser um teste de produção.
3. Antes de qualquer medição com GUI: `pgrep` + aviso ao orquestrador (feito) **e** um lembrete de
   que matar o processo não fecha alertas do sistema.
4. Publicar número em três documentos (spec, decisions, notas de release) é convite a divergir:
   escolher um dono (a evidência) e citar dela, com faixa em vez de ponto.
5. Para "primeira abertura de app baixado" em qualquer frente futura: começar pelo log do
   `CoreServicesUIAgent`, nunca por `sample` de um processo suspenso — `sample` mostra o sintoma
   (parado em `_dyld_start`) e esconde a causa (diálogo esperando).

## Dívida deixada

- Tela cheia em outro Space, cdhash nunca visto, macOS 26/Intel e Windows: não medidos, com o
  motivo na evidência.
- O modo `--click` do script commitado não foi exercitado ponta a ponta (a tabela veio do
  protótipo); a captura por rodada e a falha em 5 s também não.
- Encolher o payload (runtime em `tar.gz`, sem runtime) atacaria o custo de abrir de dentro da
  imagem; ficou fora de escopo desta issue e não foi medido.
