# Retro — Solution Explorer visual (tickets 01–04 + revisão)

## O que funcionou

- Separar o trabalho em chrome (ticket 01), tokens/ícones (02), símbolo C# (03) e Dependencies (04)
  manteve cada diff revisável; o E2E de árvore e o unitário do mapeamento pegaram regressões cedo.
- Ler o Seti/codicons só pela web (sem copiar asset) deu uma paleta defensável: contraste medido ≥ 6:1,
  `light-dark()` com fallback para o tema escuro que é o único distribuído.
- `workspace.symbolKinds` com `known`/mtime e cache no UI resolveu o limite de 256 e a piscada sem
  inventar tipo; o parser conservador (strings/comentários/BOM/stem com ponto) evita badge falso.
- A verificação no app real (agent-desktop) confirmou o que o E2E não mostra: cores, badges e
  agrupamento. As capturas estão em `docs/evidence/solution-explorer-visual/`.

## O que deu errado / o que aprender

- O E2E `zz-acceptance` tinha portas e home hardcoded; só apareceu com `OPENRIDR_E2E_*_PORT` próprios.
  Spec que importa `host`/`frontend` deve usar `tests/e2e/environment.ts`, nunca literal.
- Clicar na linha do projeto **também colapsa** a subárvore; testes que precisam do arquivo depois
  devem garantir `aria-expanded` ou usar `Expand all`.
- Descobrir testes logo após criar um `.cs` externo corre contra o modelo avaliado stale; o host
  recusa (`evaluation_stale`). Reavaliar antes é requisito, não detalhe de teste.
- A status bar tem espaço apertado: o botão de refresh novo exigiu encurtar a leitura e reduzir o gap
  para o layout de 1440 não elipsar. A elipse de `not current` foi resolvida pondo-o antes do revision.
- O parser de símbolo precisou de teste dedicado para não aceitar `Enum.Parse`, `Class`, string com
  `/*` e `record.ToString()`; a primeira versão (case-insensitive) era falsa.

## Dívida e limites

- `Assemblies` não é inventado; grupos vazios somem. `web` só é inferido do texto do `.csproj`
  (`Microsoft.NET.Sdk.Web`), não de propriedade MSBuild avaliada.
- O cache de símbolo é por path+mtime e revisão; edição de conteúdo sem reavaliação/reabertura pode
  manter o badge até a próxima sincronização.
- Tema claro continua não distribuído; os valores `light-dark()` não foram vistos no app.
- `tree-partial`/erro da status bar só têm prova E2E; foco por teclado no app real continua não
  dirigível pelo agent-desktop.

## Resultado

- `bun run check`/`build` OK; `bun test` 21/21; `cargo test -p openridr-host --lib` 235/235.
- E2E dos cinco specs tocados: **43/43 (exit 0)** nas portas 1440/4337.
- Após o merge do main (`19ccb6d`), E2E dos seis specs (inclui `focus-after-navigation`): **49/49
  (exit 0)**; `cargo test -p openridr-host`: 79 passed + a falha conhecida
  `file_access_is_confined_to_the_workspace`.
- App real: árvore sem chrome, projetos coloridos, Dependencies agrupadas, badges I/E/R/S/C e
  `no index` esmaecido; capturas em `docs/evidence/solution-explorer-visual/`.
