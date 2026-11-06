# Solution Explorer — ícones Rider Dark

## Evidência

Superfície B do verify-openridr: `web.sh` com estado isolado, host 4447 e UI 1550, OrderDesk em modo Restricted, Expand All e Show All Files, Chrome headless a 1470×880. Capturas reais:

- [Antes](rider-solution-icons/antes.png)
- [Depois](rider-solution-icons/depois.png)

Na cópia isolada da fixture foram criadas as pastas `bin`, `obj`, `config` e `.idea` antes da captura depois; a captura anterior não as contém. A árvore mostra os diretórios `bin`/`obj` como no index. O CSS computado confirmou verde `rgb(95, 184, 101)` para projeto e `rgba(120, 70, 40, 0.18)` para bin. O host e o Vite da superfície B foram encerrados.

## Validação

- `bun run check`: passou.
- `bun test tests/unit`: 33 passaram.
- Playwright solution-explorer + solution-file-icons + ui-layout, portas 1550/4447: 21 passaram e 2 falharam. `solution-file-icons`: 5 passaram. `solution-explorer` falhou em build: MSB4018, `GenerateDepsFile` não conseguiu acessar o `.deps.json` porque outro processo o estava usando. `ui-layout` 1440×900 falhou porque a label de avaliação truncou, embora o teste esperasse que não truncasse. Não foi estabelecida a causa desta última falha; não declarar a bateria verde.
- A configuração Playwright sobrepõe `OPENRIDR_HOME` usando `OPENRIDR_E2E_HOME`; nesta execução foi passado apenas `OPENRIDR_HOME`, portanto o E2E usou seu diretório temporário padrão, não o caminho solicitado. As portas foram isoladas.
- App desktop, Windows e macOS Intel: não verificados.

## Limites e decisões

A árvore não recebe estado Git por arquivo; não inventamos cores de ignorado/não rastreado. O badge azul é aplicado à pasta chamada `config`, não há dados de link na árvore para identificar symlinks. A árvore atual não expõe nó Imports; não foi possível demonstrar seu ícone. Framework usa engrenagem simples, não um par. Não foram copiados assets da JetBrains; os glifos existentes de @pierre/trees e Lucide permanecem, e o glifo Simple Icons C# foi removido com sua atribuição.

O E2E mudou as expectativas: C# como texto sem badge visual mantendo descrição acessível; pasta com SVG em vez de espaço vazio; DotSettings usa `xml` em vez de documento. O teste de contraste de documento usa um arquivo genérico em vez de DotSettings, agora XML. Unitários atualizados para projeto e DotSettings.
