# SPEC — QA médios de navegação e busca (2026-11-02)

Escopo: achados médios de `QA-BUGHUNT-2026-11-02.md` (b5, b6, b7, b8, b9, b14, b16, b19) e
`QA-DEVFLOW-2026-11-02.md` (d14, d15, d16, d17, d18). Fora: debug/run/build, editor e estado,
ferramentas e demais achados dos dois relatórios.

## 1. Find in Files (b5 / d17)

- `⇧⌘F` abre um diálogo **Find in Files** no projeto, com consulta, opção **Match case** e escopo.
- Escopo padrão: raiz do workspace. O item **Find in Files…** do menu de contexto de arquivo,
  pasta, projeto e raiz abre o mesmo diálogo com o escopo no caminho clicado; resultados fora
  do escopo não aparecem.
- Resultados agrupados por arquivo e **uma entrada por linha** (a linha pode ter várias
  ocorrências); clique/Enter abre o arquivo na linha/coluna da primeira ocorrência.
- `search.text` continua sendo a única fonte; regex e máscara de arquivo ficam de fora porque o
  host não as suporta (registrado em `decisions.md`).
- O diálogo fecha com Esc e o campo de consulta recebe foco ao abrir.

## 2. Go to Line (d16 / b19)

- `⌘G` e `⌘L` (perfil Rider) e `⌘L` (IntelliJ) abrem o Go to Line do CodeMirror, com a linha
  atual preenchida; Enter move o cursor e rola até a linha.
- A ação `search.line` deixa de ser stub e funciona com foco no editor e fora dele (foca o
  editor antes de abrir).

## 3. Histórico Back/Forward (d15)

- Navegações do usuário (abrir arquivo pela árvore, busca, menu, problemas, teste, frame de
  debug e Go to Definition/Find Usages) entram num histórico de `{path, line, character}`.
- `⌘[` / `⌘⌥←` voltam; `⌘]` / `⌘⌥→` avançam. Os botões Back/Forward da toolbar refletem
  `canBack`/`canForward` e executam a mesma ação.
- Nova navegação depois de voltar descarta o ramo à frente; o histórico é limitado a 100
  entradas e não é persistido entre sessões. Abrir o arquivo inicial não cria histórico.

## 4. Abas, Recent Files e Go to File (d17 / b19)

- `⌘W` fecha a aba ativa (com o diálogo de buffer sujo já existente).
- `⇧⌘]` / `⌃Tab` próxima aba; `⇧⌘[` / `⌃⇧Tab` aba anterior, com wrap.
- `⌘E` abre **Recent Files** (lista da sessão, mais recente primeiro, até 20); clique/Enter
  abre o arquivo; Esc fecha.
- `⇧⌘O` (Rider) abre o Go to File existente (Search Everywhere na aba Files). `⌘T` deixa de
  ser Search Everywhere no perfil Rider; `⇧⇧` continua abrindo.
- Format e Organize Usings **não entram**: o host não expõe `textDocument/formatting` nem
  `source.organizeImports`; a limitação fica em `decisions.md`.

## 5. Perfil Rider (d18)

- Só no perfil `rider-macos`: `⌘D` duplica a linha e `⌘⌫` apaga a linha. O perfil IntelliJ
  mantém o comportamento atual do CodeMirror.
- `⌘T` sai do perfil Rider (Search Everywhere fica em `⇧⇧` e Go to File em `⇧⌘O`).

## 6. Duplo Shift (b16)

- O Search Everywhere só abre com **duas batidas limpas** de Shift (keydown+keyup sem outra
  tecla no meio) dentro de 400 ms. `Shift+↑` seguido de `Shift+Home` não abre.
- Texto em input/textarea/select/contenteditable continua isento.

## 7. Diálogos Push/Update (d14)

- Abrir Push (`⇧⌘K`) ou Update (`⌥⌘W`) por atalho foca o botão de confirmação; digitar não
  altera o buffer; Esc fecha o diálogo.

## 8. Search Everywhere (b6 / b7)

- Text/All: uma entrada por arquivo+linha, sem duplicatas por ocorrência na mesma linha.
- All ordena: arquivo com nome exato, símbolo com nome exato, arquivo com prefixo, símbolo com
  prefixo, ações, demais arquivos/símbolos, texto; texto não inunda o topo.
- A aba Symbols inclui tipos e membros (uma classe aparece em Symbols e em Classes); Classes
  continua só com tipos. O rótulo de tipo na lista é `class`, membro é `symbol`.
- Reabrir Search Everywhere volta para **All**, mantém a consulta anterior **selecionada**
  (digitar substitui, não anexa).

## 9. Find Usages e notificações (b9)

- O preview docado do Find Usages não reduz a faixa inferior a 96 px em janelas normais
  (≥ 700 px de altura): a tool window mantém a altura configurada e o preview cabe no editor.
- Notificações (`workspace-notices`) não empurram abas/editor: sobrepõem o topo do editor.

## 10. Workspace tools (b8)

- O drawer não rola na horizontal: comandos, caminhos e textos longos quebram dentro da
  largura; `scrollWidth <= clientWidth` em 1440×900 e 1100×700.

## Testes

- Um spec Playwright por ticket em `tests/e2e/medium-nav-*.spec.ts`, escrito antes da correção,
  contra host real e `fixtures/OrderDesk` (cópia temporária quando houver edição).
- Testes Rust só se o host mudar; nenhuma mudança de host está prevista.
- Specs que dependem de csharp-ls/netcoredbg fazem `skip` honesto quando a ferramenta não está
  no home E2E.
