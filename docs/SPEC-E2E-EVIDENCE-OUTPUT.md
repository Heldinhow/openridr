# SPEC — A captura do E2E não escreve no repositório por default (issue #217)

Status: ready-for-agent
Fonte: issue GitHub #217 (Heldinhow/openridr), encontrada no retro da #210 + grill de 2026-11-07 com o orquestrador (Q1–Q8).
Tickets: issues filhas de #217 (criadas por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

Treze specs do E2E calculam o diretório das capturas como
`OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots"` — ou seja, **o default aponta para dentro do
repositório**. O diretório está no `.gitignore`, mas **86 PNGs dele estão versionados** (o `.gitignore`
não vale para arquivo já rastreado), então uma execução que falha no meio do caminho **reescreve
evidência versionada sem ninguém pedir**. Aconteceu duas vezes durante a #210 e deixou um
`git worktree remove` recusando apagar um checkout descartável.

O sintoma para quem trabalha: depois de um spec que falha, `git status` mostra arquivos alterados que
a sessão não alterou, e não há como distinguir o próprio diff do estrago da execução. A saída atual é
lembrar de exportar `OPENRIDR_SCREENSHOT_DIR` **antes** de rodar — uma regra que o `AGENTS.md` e o
skill `verify-openridr` precisam repetir porque o default é perigoso.

Dois specs pioram a mesma classe de problema: gravam num caminho literal dentro do repositório e
**ignoram a variável**, então nem a saída manual funciona neles.

## Solution

O default do diretório das capturas sai do repositório: passa a ser um diretório temporário
**derivado da porta da UI**, no mesmo molde do E2E app home da #210. O caminho versionado
(`docs/evidence/screenshots`) continua existindo e continua sendo usado — mas só quando
`OPENRIDR_SCREENSHOT_DIR` aponta para ele de propósito.

Com isso, uma execução que falha deixa o `git status` limpo, duas sessões em portas diferentes não
sobrescrevem as capturas uma da outra, e o `AGENTS.md` e o skill `verify-openridr` deixam de avisar
sobre um default que não existe mais. Um teste em CI impede que a próxima sessão reintroduza o
caminho versionado como default.

## User Stories

1. Como sessão de agente rodando um spec que falha, quero que as capturas caiam fora do repositório, para o meu `git status` continuar limpo e eu não confundir o estrago da execução com o meu diff.
2. Como sessão de agente, quero distinguir com um olhar o que eu mudei do que a execução escreveu, para não commitar nem reverter arquivo errado.
3. Como sessão de agente que precisa de evidência versionada, quero apontar `OPENRIDR_SCREENSHOT_DIR` para `docs/evidence/screenshots` e ter as capturas lá, para o caminho versionado ser escolha minha e não default.
4. Como sessão de agente, quero que o default seja derivado da porta da UI, para duas sessões em portas diferentes não sobrescreverem as capturas uma da outra.
5. Como sessão de agente, quero que o caminho default seja previsível e documentado, para achar a captura de uma falha sem procurar no disco inteiro.
6. Como sessão de agente, quero que `OPENRIDR_SCREENSHOT_DIR` relativo continue resolvido contra o diretório atual, para nada mudar para quem já passa caminho relativo.
7. Como sessão de agente, quero que **todos** os specs respeitem a variável, para não descobrir depois que um deles ignorou e sujou a árvore.
8. Como sessão de agente, quero que os dois specs que gravam em caminho literal passem a usar o mesmo default, para não existir uma segunda regra escondida.
9. Como sessão de agente que acabou de rodar um spec, quero poder apagar o checkout descartável com `git worktree remove`, para não ficar com worktree preso por causa de um PNG reescrito.
10. Como sessão de agente escrevendo um spec novo, quero um helper único para o diretório das capturas, para não copiar a linha mágica em cada arquivo.
11. Como mantenedor, quero um teste que rode no CI e falhe se um spec voltar a nomear o caminho versionado como default, para a classe do bug não voltar em silêncio.
12. Como mantenedor, quero o teste do default como função pura (porta + override), para provar o comportamento sem abrir navegador.
13. Como mantenedor, quero que o smoke noturno continue rodando sem variável nova, para o workflow não virar um lugar onde a configuração mora.
14. Como mantenedor, quero o `AGENTS.md` e o skill `verify-openridr` sem o aviso sobre o default antigo, para a documentação não ensinar um perigo que já foi resolvido.
15. Como mantenedor, quero a decisão sobre `outputDir`/`playwright-report` e sobre `OPENRIDR_EVIDENCE_LOG_DIR` registrada, para não ser reaberta por engano.
16. Como revisor, quero a prova medida (execução que falha, `git status` limpo, capturas no temporário) em `docs/evidence/`, para não precisar repetir a medição.
17. Como sessão futura, quero que os PNGs versionados continuem versionados e intocados, para esta frente não apagar evidência histórica.
18. Como mantenedor, quero zero dependência nova e zero mudança de produto, para a issue ser só do harness.

## Implementation Decisions

- **Default = `<tempdir>/openridr-e2e-screenshots-<uiPort>`.** A chave é a porta da UI, o mesmo critério
  do E2E app home (`openridr-e2e-home-<uiPort>`, #210): duas execuções em portas diferentes não
  compartilham diretório, e duas na mesma porta não coexistem de qualquer forma (o Vite falha rápido
  com `strictPort`). `OPENRIDR_SCREENSHOT_DIR` continua vencendo, e um valor relativo continua
  resolvido contra o diretório atual — nada muda para quem já passa caminho.
- **Um seam só: o módulo de ambiente do E2E.** Ele já é o ponto por onde o config do Playwright, o
  `globalSetup` e todos os specs leem portas, home e URLs. Ganha uma função pura
  `screenshotsDirFor(uiPort, override)` mais a constante derivada, e os specs passam a importar daí em
  vez de calcular o caminho localmente. Nenhum módulo novo, nenhuma camada nova.
- **Os dois specs que ignoram a variável passam a respeitá-la.** Eles gravam uma captura de um estado
  que só interessa quando o spec falha, então continuam gravando sempre — só que no default
  compartilhado, e não num caminho literal dentro do repositório.
- **Os specs que já são opt-in ficam opt-in.** Os três que só capturam quando a variável está setada
  não ganham default: eles nunca escreveram sem ser pedidos e não há o que corrigir.
- **`outputDir` e `playwright-report` ficam onde estão.** `test-results/` e `playwright-report/` já
  estão no `.gitignore` e não têm arquivo versionado, então não sujam `git status` — mover para o
  temporário não resolve problema nenhum e muda caminho que gente já conhece. Decisão registrada, não
  esquecimento.
- **`OPENRIDR_EVIDENCE_LOG_DIR` fica fora.** O default dele (`docs/evidence/logs`) é ignorado e não tem
  arquivo versionado: não suja `git status`. A issue é sobre captura de tela; log de evidência fica
  como está, e a decisão fica escrita para não ser reaberta.
- **Os 86 PNGs versionados não são movidos nem apagados.** Continuam versionados e intocados; o que
  muda é que ninguém escreve neles por default.
- **Docs vivos corrigidos, históricos preservados.** Saem o aviso e o pré-requisito do default antigo
  do `AGENTS.md`, do skill `verify-openridr` e do bloco de pré-requisitos do smoke em
  `docs/evidence/e2e-quarantine.md`. Retro e evidência de execuções passadas (`RETRO-E2E-PORTS`,
  `evidence/e2e-ports.md`, `solution-explorer-visual.md`, `28-rider-visual-pass.md`) ficam como
  registro da época, sem reescrita.
- **Sem dependência nova, sem mudança de produto, sem `data-testid`.** Nenhum comando de host, nenhum
  CSS, nenhuma variável nova de ambiente.

## Testing Decisions

- **Bom teste aqui prova onde a captura cai**, não como o módulo é montado: qual diretório o default
  produz, que o override vence, e que nenhum spec nomeia o caminho versionado. Nada de espiar detalhe
  interno do helper.
- **Seam único (existente): o módulo de ambiente do E2E.** A função pura é testada em `tests/unit`, que
  é o que o CI roda (`bun test tests/unit`); o spec de Playwright que já cobre esse módulo continua
  cobrindo o resto.
- **Guarda contra a reincidência.** O teste em `tests/unit` varre os specs do E2E e recusa o caminho
  versionado como literal — é o que fecha a classe do bug, inclusive os dois specs que hoje ignoram a
  variável.
- **Falha-antes/passa-depois:** a guarda falha na árvore de hoje (treze defaults mais dois literais) e
  passa depois; a prova real é uma execução que falha com o default antigo sujando `git status` e a
  mesma execução com o default novo deixando limpo.
- **Aceitação (os critérios da issue):** (1) execução que falha, sem `OPENRIDR_SCREENSHOT_DIR`, deixa
  `git status --porcelain` vazio e as capturas em `<tempdir>/openridr-e2e-screenshots-<porta>`;
  (2) com `OPENRIDR_SCREENSHOT_DIR` apontando para um diretório, as capturas vão para lá; (3) o
  `AGENTS.md` e o skill não avisam mais sobre o default antigo.
- **Prior art:** o teste de `e2eHomeFor` em `tests/e2e/environment.spec.ts` (função pura de ambiente) e
  os testes de unidade que afirmam sobre conteúdo de arquivo do repositório (`readme.test.ts`,
  `communityReportingDocs.test.ts`).
- **Nada de suíte completa por reflexo:** os specs tocados pela frente são os que gravam captura; o
  resto continua no smoke noturno.

## Out of Scope

- `OPENRIDR_EVIDENCE_LOG_DIR` e o `outputDir`/`playwright-report` do Playwright (decididos acima: ficam).
- Consertar os specs quarentenados (#43/#67): `debugger`, `csharp-ls:150`, `testsuite`, `tests:317`,
  `ui-layout` em 1440 px, `find-usages`, `run-configs`, `tests`. Continuam fora do verde.
- Apagar, mover ou re-versionar os 86 PNGs de `docs/evidence/screenshots`.
- Rodar Playwright em PR ou mudar o smoke noturno.
- Windows e macOS Intel.

## Further Notes

- **Por que a porta e não o worktree.** Mesmo motivo do E2E app home: a porta é a única coisa que a
  sessão escolhe, e duas execuções no mesmo worktree com portas diferentes também precisam de
  diretórios diferentes.
- **Por que não `mkdtemp` por execução.** Ninguém acha a captura depois sem o log; o caminho derivado da
  porta é previsível e documentado.
- **O custo aceito.** O diretório temporário acumula capturas entre execuções na mesma porta; o
  sistema limpa o temporário, e nada no repositório depende disso.
- **Risco do PR.** A frente toca `AGENTS.md`, que é `risk:high`: o merge é do mantenedor, e a sessão
  para depois do PR verde.
