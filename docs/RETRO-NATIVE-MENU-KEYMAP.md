# Retro — menu nativo × keymap (2026-11-06, issue #111)

Frente: #111 (filha de #50), tickets #120 (menu), #129 (tecla com Option) e #121 (aceitação).
Spec: [SPEC-NATIVE-MENU-KEYMAP](SPEC-NATIVE-MENU-KEYMAP.md). Evidência:
[evidence/native-menu-keymap.md](evidence/native-menu-keymap.md).

## O que funcionou

- **Medir o menu real antes de decidir.** Dois comandos (`agent-desktop snapshot --surface menubar` e
  `System Events` para `AXMenuItemCmdChar`) mostraram que o `Close All` **não vem do Tauri**: o AppKit
  injeta o item no menu File a partir do item com seletor `performClose:`, e deriva o acelerador do
  original. Sem esse fato, a correção óbvia seria "remover o Close All depois do launch" — frágil.
- **Spike descartável para a pergunta do sistema.** Um app Swift de 40 linhas em `/tmp` respondeu, em
  duas execuções, o que nenhum teste de unidade responderia: `performClose:` + ⌘W gera `Close All` com
  ⌥⌘W; sem acelerador o injetado nasce sem tecla; sem item, ele não aparece. Custo: minutos. Alternativa
  (mexer no app e rebuildar para descobrir) custaria três builds.
- **A tecla foi provada, não deduzida.** `⌘W`/`⌥⌘W` antes da correção mataram o PID do app instalado; as
  mesmas teclas no bundle corrigido produziram o efeito do keymap. "Falha-antes" aqui é um PID morto, não
  um raciocínio.
- **Fixar o que o ambiente compõe.** A segunda metade do bug (`⌥W` chega como `"∑"`) só apareceu porque a
  aceitação rodou de verdade: o menu já não sombreava nada, `⌘W` funcionava, `⌥⌘W` continuava mudo. Um
  experimento de dez linhas com `matches()` fechou o diagnóstico em vez de abrir uma caça ao tesouro no
  React.
- **Guarda onde o E2E não alcança.** O E2E roda em Chrome, sem menu nativo; a colisão de atalho só é
  visível no shell. A lista versionada (`reservedShortcuts.ts`) + `bun test` é o único lugar barato para
  prender isso, com a mensagem já nomeando ação, perfil e tecla.

## O que quase passou batido

- **`bun run check` não compila o shell.** `src-tauri` é um workspace Cargo separado, então
  `tsc --noEmit && cargo check --workspace --all-targets` não vê `main.rs` — e o hook de pre-commit roda
  só `bun run check`. Uma mudança no menu passaria pelo hook sem ser compilada. Nesta frente rodei
  `cd src-tauri && cargo check` na mão; o hook continua cego.
- **`cargo check` na shell exige os recursos vendorizados.** O build script valida
  `resources/{dotnet,csharp-ls,netcoredbg}`; numa worktree nova, `cargo check` falha com
  `resource path resources/dotnet doesn't exist` até rodar `scripts/vendor-*.sh` (que baixa ~100 MB).
- **O atalho errado já estava "documentado como funcionando".** `⌘W` de Close Tab constava em spec desde
  a onda de navegação e a evidência da alpha.5 dizia "listado, não executado". Ninguém executou porque
  executar fechava o app.
- **A sombra do menu escondia a segunda falha.** Enquanto o item nativo consumia `⌥⌘W`, o defeito de
  composição do Option era invisível. Corrigir metade e rodar a tecla foi o que revelou a outra metade —
  lição: depois de tirar um bloqueio, repetir o caminho do usuário antes de declarar vitória.

## Revisão (o que a rodada 1 pegou)

- **A tecla física roubava digitação.** Ligar `alt` ao `event.code` consertou o comando e quebrou a
  digitação: `tests.quickList` é ⌥⇧U, e ⌥⇧U é tecla morta/composta no macOS. O handler do editor dava
  `preventDefault` e o caractere deixava de entrar. Regra nova: com foco no CodeMirror, atalho **só de
  Option** (sem ⌘/⌃) que compõe outro caractere cede ao editor; fora dele a ação roda. Lição: ao
  "consertar" o mapeamento de teclas, o caminho de **entrada de texto** é um consumidor de primeira
  classe, não um caso de borda.
- **Compatibilidade com override antigo.** O matcher novo olhava só a tecla física; capturas antigas
  guardaram o caractere composto (`"∑"`), e esses bindings parariam sem aviso. `matches` aceita as duas
  formas, e a afirmação "nenhum override precisa mudar" passou a ser verdade — antes era só otimismo.
- **O que a revisão confirmou bem.** Fluxo de fechar janela com buffer sujo sem caminho novo de descarte,
  `close()` como o único fechamento programático, e a guarda de menu assumidamente um espelho manual (a
  spec já dizia que ela não prova mudanças futuras do menu nativo). `bun test tests/unit` = 59 testes
  depois das correções. O revisor não rodou app nem `cargo check` — essa parte ficou com a verificação
  real (S7–S9), que é onde o achado de digitação poderia ter sido visto antes.

## Incidentes operacionais

- **Duas sessões no mesmo state de verificação.** `desktop.sh` usa por default
  `$TMPDIR/verify-openridr-<uid>-desktop`; a sessão da #112 subiu depois do meu `down` e escreveu o PID
  dela no mesmo `app.pid`. Um `down` meu teria derrubado o app alheio (e o contrário também). Regra que
  fica: quem roda em paralelo exporta `VERIFY_OPENRIDR_DESKTOP_STATE` próprio antes do `up`; o `pgrep`
  sozinho não basta porque o bundle tem o mesmo `CFBundleIdentifier` em todas as worktrees.
- **`⌘Q` e `⌘W` exigem `--force` no agent-desktop.** Sem isso o adapter recusa o combo como perigoso; a
  prova de saída só aconteceu com `press --force "cmd+q"`.
- **`type`/`press` precisam de foco.** `type` só entregou depois de `focus-window` + `--headed`; sem foco
  o erro é "no verified focused element", não um bug do app.

## Custo/benefício

- Mudança de produto: ~35 linhas no shell (substituir dois itens e tratar um id), ~20 linhas no matcher,
  dois testes de unidade e uma lista de dados. Zero dependência nova, zero `data-testid` novo (não há UI
  nova), nenhum atalho/padrão alterado.
- O `Close All` desaparece sem código dedicado porque a causa era o item `performClose:` — uma correção
  de causa, não um `remove` defensivo.
- Custo real da frente foi diagnóstico (spike AppKit + leitura do menu), não implementação.

## Encaminhamentos

- **Hook de pre-commit não cobre o shell.** Candidato a ticket: incluir `cargo check` de `src-tauri` no
  `bun run check` (ou num hook separado), com cache de `target/` para não pagar minutos. Sem isso, todo
  ticket que toca `main.rs` depende de alguém lembrar do comando.
- **Windows/Linux podem ter a mesma sombra** (`Menu::default` com fechar janela acelerado) e não são
  suportados nem testados: registrado como não verificado, não como corrigido.
- **Layouts não-US** com Option passam a nomear o atalho pela posição física da tecla; se algum dia o
  repo suportar ABNT2/AZERTY, vale um teste com `code` diferente do `key`.
- **`delegate_task` do code review**: a primeira listagem de `orchestrator_capabilities` que li vinha
  filtrada (`omittedModels: 32`) e escondia `openai-codex/gpt-6.1-terra` no provider Pi; eu concluí
  "não existe" e disparei a revisão em `gpt-6.1-sol`. O orquestrador corrigiu, o run foi cancelado e a
  revisão rodou no modelo pedido (`openai-codex/gpt-6.1-terra`, thinking medium). Lição: catálogo
  filtrado não é catálogo inteiro — buscar pelo id antes de trocar de modelo.
