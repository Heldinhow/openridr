# SPEC — Menu nativo não engole atalho do keymap (issue #111)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe: [#111](https://github.com/Heldinhow/openridr/issues/111), filha de [#50](https://github.com/Heldinhow/openridr/issues/50).

## Problema

O keymap é a fonte única dos atalhos do produto: a Settings, a Find Action, os menus da UI e os botões leem o mesmo registro, e o `window` do webview captura `keydown` em fase de captura para executar a ação. Mas o macOS resolve **key equivalent de item de menu nativo antes de entregar a tecla ao webview**. Quando um item do menu nativo usa a mesma combinação, o atalho anunciado pelo app nunca chega ao app.

Hoje há exatamente duas colisões, medidas contra o menu real e contra a tabela de defaults dos três perfis:

| Ação | Atalho anunciado | Item nativo que o consome |
|---|---|---|
| `editor.closeTab` ("Close Tab") | ⌘W | File > Close Window (⌘W) — e a duplicata em Window > Close Window |
| `git.update` ("Update Project…") | ⌥⌘W | File > Close All (⌥⌘W), injetado pelo AppKit |

Consequência do `git.update`: pressionar o atalho do Update executa **Close All**, e com uma janela só o app **encerra** — a issue #111 nasceu daí (rodada de aceitação do Git remoto, [#50](https://github.com/Heldinhow/openridr/issues/50)).

Consequência do `editor.closeTab`, descoberta nesta sessão e pior: **⌘W também encerra o app**. Reproduzido no app instalado (0.3.0-alpha.9): com a janela em foco, `⌘W` derrubou o PID, sem nenhum diálogo. O atalho de fechar aba é documentado desde a [SPEC-MEDIUM-NAV-SEARCH](SPEC-MEDIUM-NAV-SEARCH.md) e nunca foi executado numa prova real — a linha do `evidence/e2e-alpha5/REPORT.md` diz "listado, não executado".

O bug não é do webview nem do keymap: é do menu nativo. O `editor.closeTab` está quebrado desde que existe, e o `⌥⌘W` desde que o Update ganhou atalho.

## Solução

O menu nativo deixa de possuir as duas combinações. `⌘W` e `⌥⌘W` voltam a chegar ao webview e executam a ação do keymap; o menu nativo fica só com os atalhos que **são** do sistema (`⌘Q`, `⌘H`, `⌥⌘H`, `⌘M`, edição, `⌃⌘F`).

Para isso, a cirurgia é no item que cria a sombra:

- **`Close Window` deixa de ser o item predefinido com acelerador.** O item continua no menu File, mas como item próprio, sem `accelerator`, com id tratado no shell, que fecha a janela pelo mesmo caminho do botão vermelho (o `CloseRequested` que a UI já intercepta para perguntar sobre buffer sujo).
- **`Close All` desaparece.** Ele não é nosso: o AppKit o injeta no menu File porque existe um item com o seletor `performClose:`. Removendo esse item, o Close All nasce, se nascer, sem tecla — e, como na prática o item de File passa a ser um item comum, ele nem aparece. Num app de janela única ele não tem função: sair é `⌘Q`, que já pergunta.
- **A duplicata de fechar janela no menu Window sai.** Sobra uma única entrada, em File.

O keymap não muda: `⌘W` continua "Close Tab" e `⌥⌘W` continua "Update Project…", nos três perfis. A paridade com o Rider (⌘W fecha aba, ⌥⌘W faz update) fica intacta; quem fecha a janela pela janela do app é o item de menu, o botão vermelho ou `⌘Q`.

Uma **guarda** impede a recaída: a lista de aceleradores que o menu nativo possui passa a ser dado versionado no lado TS (comentado contra o código do shell) e um teste de unidade reprova qualquer default do keymap que colida com ela. E2E não cobre isso — roda em Chrome, sem menu nativo.

## User stories

1. Como pessoa usando o OpenRidr, quero que `⌥⌘W` abra o diálogo de Update, para que o atalho anunciado na Find Action e na Settings seja o que acontece.
2. Como pessoa usando o OpenRidr, quero que `⌘W` feche a aba ativa, para não perder a janela (e o app) ao tentar fechar um arquivo.
3. Como pessoa usando o OpenRidr, quero que `⌘W` numa aba com edições não salvas pergunte antes de descartar, para não perder trabalho.
4. Como pessoa usando o OpenRidr, quero poder fechar a janela pelo menu File, para ter o caminho explícito sem atalho.
5. Como pessoa usando o OpenRidr, quero que `⌘Q` continue perguntando quando há buffer sujo, para não perder trabalho ao sair.
6. Como pessoa usando o OpenRidr, quero que o botão vermelho da janela continue perguntando quando há buffer sujo, para que fechar pelo mouse seja tão seguro quanto sair por `⌘Q`.
7. Como pessoa usando o OpenRidr, quero um menu File sem "Close All", para não ter um comando de app multi-janela num app de janela única.
8. Como pessoa usando o OpenRidr, quero que os perfis Rider, IntelliJ e Visual Studio tenham os mesmos atalhos depois da correção, para não descobrir um perfil quebrado depois.
9. Como pessoa que mantém o OpenRidr, quero que a lista de atalhos do menu nativo esteja num teste, para que a próxima colisão seja pega antes do app.
10. Como pessoa que mantém o OpenRidr, quero que o teste diga qual ação e qual atalho colidiram, para corrigir sem investigar o menu à mão.
11. Como pessoa que revisa o código, quero que o motivo da cirurgia no menu esteja num comentário no código do shell e em `decisions.md`, para não "consertar" isso de volta para `Menu::default` sem perceber.
12. Como pessoa que mantém o OpenRidr, quero a prova no app Tauri real com o bundle sob teste, não no navegador, porque o bug só existe no shell.
13. Como pessoa que valida a release, quero a evidência com ação e estado resultante (app vivo, aba fechada, diálogo aberto, menu lido), para auditar sem repetir a sessão.
14. Como pessoa que valida a release, quero que o que não foi verificado (Windows, macOS Intel) esteja escrito, para não vender suporte que não existe.
15. Como pessoa usando o OpenRidr no Windows ou Linux, quero saber que o mesmo problema pode existir lá, para não tratar a correção como universal.

## Implementation decisions

- **A cirurgia é no shell (`src-tauri`), não no keymap nem no webview.** O `default_menu` do macOS continua partindo de `Menu::default` (como já faz ao trocar o Quit), e passa a: (a) trocar o item predefinido de fechar janela do menu File por um `MenuItem` próprio, com id e **sem** acelerador; (b) remover o item predefinido de fechar janela do menu Window (a duplicata); (c) tratar o id do item próprio no `on_menu_event`, fechando a janela pelo método que emite `CloseRequested` — o mesmo caminho do botão vermelho, que a UI intercepta para perguntar sobre buffer sujo. Não se usa `destroy`, que não emite evento.
- **Por que o Close All some sem tocar nele.** O AppKit injeta `Close All` (seletor `closeAll:`, ⌥⌘W) no menu File porque o menu File contém um item com o seletor `performClose:`, e o acelerador do item injetado é derivado do acelerador do item original. Medido num spike AppKit descartável (fora do repo): com `performClose:` + ⌘W o Close All nasce com ⌥⌘W; com `performClose:` sem acelerador ele nasce sem tecla; sem nenhum item `performClose:` ele não aparece. Um item próprio (sem seletor) cai no terceiro caso — por isso a solução é trocar o item, e não tentar apagar o injetado depois.
- **O keymap não muda.** `editor.closeTab` continua ⌘W e `git.update` continua ⌥⌘W nos três perfis; nenhum default, nenhum override, nenhuma migração de `settings.json`. A colisão é resolvida do lado do menu nativo.
- **`⌘Q` continua nativo e continua interceptado.** O item Quit já é substituído por um item próprio que pede a decisão à UI (`quit_app` só depois do prompt); nada nesta frente toca esse fluxo.
- **Menu File mantém "Close Window" sem atalho; menu Window perde a duplicata.** A janela continua fechável por menu e pelo botão vermelho; o teclado de fechar janela passa a ser do app (`⌘W` = fechar aba), como num IDE.
- **Guarda do lado TS, com a lista nativa versionada.** Um módulo de dados exporta os aceleradores que o menu nativo possui, cada um com comentário de onde vem (`src-tauri`, `Menu::default` e a substituição do `⌘Q`), e um teste de unidade reprova qualquer default de keymap (nos três perfis) que colida com a lista, com mensagem nomeando ação e atalho. A lista é espelho manual por construção: o shell não exporta a sua tabela e o AppKit não é inspecionável em teste de unidade; o teste é uma **armadilha de regressão**, não uma prova do menu.
- **Sem dependência nova.** Nada de crate, plugin ou pacote; só `tauri::menu` e o `bun test` que já existem.
- **Só macOS.** A ramificação não-macOS continua com `Menu::default`. O mesmo sombreamento é *possível* em Windows/Linux (o menu default tem fechar janela com acelerador), mas essas plataformas não são suportadas nem verificadas neste repo: fica declarado como não verificado, sem mudança de código.
- **Registro da decisão em `decisions.md`** com o mecanismo do AppKit e a alternativa recusada (remapear o keymap), para ninguém reverter para `Menu::default` nem "consertar" trocando `⌘W` de tecla.

## Testing decisions

- **Um teste bom aqui prova o efeito no app, não o relato.** A prova primária é o app Tauri real: a tecla é pressionada de verdade e o estado resultante é lido (aba fechada, diálogo aberto, app vivo, menu lido pela árvore de acessibilidade), porque o bug só existe onde existe menu nativo.
- **A costura mais alta disponível para automatizar é o contrato do keymap.** `tests/unit` (bun test, DOM simulado como em `nativeMenu.test.ts`) é o lugar da guarda: dados puros + `shortcutsFor`/`signature`, sem Tauri, sem AppKit.
- **A construção do menu não vira teste de unidade.** `default_menu` exige `AppHandle` e o comportamento que importa (key equivalent do AppKit, item injetado) não existe no runtime de teste. Em vez de um mock que fingiria o AppKit, a prova do menu é a leitura do menu real no app.
- **Prior art**: `tests/unit/nativeMenu.test.ts` (guarda de comportamento de shell exercitada contra DOM mínimo) e `docs/evidence/native-context-menu.md` (app real dirigido por `agent-desktop`, menu lido pela árvore).
- **Falha antes / passa depois**: antes da correção, `⌥⌘W` e `⌘W` matam o app no bundle instalado (reproduzido nesta sessão); depois, as mesmas teclas no bundle sob teste deixam o app vivo e produzem o efeito do keymap. A guarda de unidade não é falha-antes — é armadilha para a próxima colisão, e isso está dito no ticket.
- **Cenários no app real (S1–S6)**: (S1) `⌥⌘W` solta o diálogo de Update com o app vivo; (S2) `⌘W` com duas abas fecha só a ativa; (S3) `⌘W` na aba com edição não salva pergunta; (S4) `⌘Q` com edição não salva pergunta e o app segue vivo até a decisão; (S5) botão vermelho com edição não salva pergunta; (S6) menu File lido sem `Close All` e com `Close Window` sem atalho.
- **O que não rodou é declarado**: Windows, Linux e macOS Intel ficam como não verificados, com motivo.

## Out of scope

- Remapear acelerador de item nativo a partir da Settings (só o keymap é remapeável).
- Recriar o menu nativo inteiro à mão (mantém-se `Menu::default` como base).
- Devolver um `Close All` funcional ou qualquer recurso de múltiplas janelas.
- Mudar o keymap, os perfis ou os atalhos do Rider/IntelliJ/Visual Studio.
- Windows e Linux (o menu continua `Menu::default`), macOS Intel.
- Qualquer outro item do menu nativo (Edição, View, Help).

## Further notes

- A medição que originou a spec, feita no app instalado 0.3.0-alpha.9 e num spike AppKit descartável em `/tmp` (não versionado): menu File com `Close Window` ⌘W e `Close All` ⌥⌘W; `⌘W` e `⌥⌘W` derrubam o PID; o Close All injetado depende do item `performClose:` do menu File; remover o item depois do launch também funciona, mas a troca declarativa é mais simples de manter.
- A lista de aceleradores do menu nativo pós-correção, para o teste: `⌘Q` (Quit, substituído), `⌘H`/`⌥⌘H`/`⌘M` (AppKit), `⌘Z`/`⇧⌘Z`/`⌘X`/`⌘C`/`⌘V`/`⌘A` (Edição), `⌃⌘F` (View), mais os itens automáticos do AppKit no menu Window (`⌘`` para ciclar janelas) e `⌃⌘Space` (Emoji & Symbols) — nenhum deles é usado pelo keymap hoje.
- `docs/MAPA.md` ganha o ponteiro para onde vive a decisão de menu (o shell), porque hoje o assunto "menu nativo" só aparece ligado ao guard de contextmenu do webview.
