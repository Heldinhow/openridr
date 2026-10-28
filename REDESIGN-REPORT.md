# Redesign — correção pós-review

Este arquivo não existia na worktree recebida. Registro restrito à correção sobre `5031a5a`; a validação anterior de 51/51 foi informada no handoff, não reexecutada antes da correção.

## Standards

- Revisão inicial recebida: **1 hard P2 + 1 smell P3**.
- **Hard P2 corrigido (1/1):** restaurados exatamente os `title` condicionais anteriores de Configuration/TFM, agora na toolbar. Labels, callbacks e gates `!evaluationAvailable || evaluationBusy` permanecem iguais. Os textos explicam trust/SDK e descrevem a função quando disponível.
- Cobertura incremental nos testes existentes: ambos os controles com SDK resolvido, após revogar trust e com SDK ausente. Nenhum assert anterior removido.
- **Smell P3 adiado explicitamente (1):** manter os pequenos handlers locais de navegação das duas tablists evita uma abstração com utilidade mínima; não é defeito funcional nem bloqueia esta correção. Reavaliar apenas se surgir outro consumidor ou divergência comportamental.
- Totais após correção: **0 hard pendentes; 1 smell aceito/adiado**.

## Spec

- Revisão inicial recebida: **0 findings**. Sem nova funcionalidade ou mudança de gate.
- O motivo textual restaurado atende `docs/SPEC.md:100` e `docs/implementation-notes.md:160`, dentro da opção de titles originais aceita para esta correção.
- Após instrução posterior explícita, `RETRO-REDESIGN.md` recebeu apenas o status factual final do P2. O arquivo ainda estava ausente ao finalizar; nenhuma recomendação de outro agente foi removida ou reescrita.

## Commits

Histórico preservado, sem squash, checkout adicional, push, PR ou merge:

- `55785c2` — shell compacto e toolbar.
- `fd6ae58` — explorer redimensionável e inspector sob demanda.
- `6d66edd` — tabs e avisos de workspace.
- `f63ba82` — painéis e navegação por teclado.
- `3233050` — welcome e toolchain recolhível.
- `79b8e7a` — status e posição do cursor.
- `bceeeb1` — polimento e verificação responsiva.
- `b123b1b` — scroll local do editor vazio.
- `5031a5a` — portas E2E isoladas e bridge same-origin (base recebida).
- `9aa95d3` — correção P2 e asserts de regressão.

Resultados desta execução e caminhos de evidência: `VALIDATION-NOTES.md`.

## Resultado final desta execução

- Build: **PASS**, antes e depois da suíte (warning de chunk >500 kB preservado).
- Full E2E: **51 passed (5.1m)**, sem falhas ou skips; inclui os asserts novos de trust/SDK.
- Standalone posterior: **2 passed (8.2s)**; **6/6 screenshots** gerados e lidos (welcome, workspace vazio e editor; 1440×900 e 1100×700). Sem corte de ações, overflow horizontal ou regressão visual aparente; o teste também prova scroll local, resize e teclado.
- Backup anterior e arquivo de evidências pós-full preservados fora do checkout. **57 arquivos de evidência** sobrescritos pelos testes foram restaurados byte a byte do backup, após verificar que a versão inicial correspondia ao HEAD e que a atual ainda correspondia à execução. Fixtures versionadas permaneceram sem diff.
- Os PIDs próprios 29884/29995/30036 já encerraram; portas 15420/15317 livres após o standalone. Nenhum kill de processo alheio, push ou mudança de checkout.
- Limites: não houve nova validação do app desktop nativo, nem teste manual de tooltip com leitor de tela; a regressão é coberta pelos titles originais e asserts de DOM. P3 permanece deliberadamente adiado.
