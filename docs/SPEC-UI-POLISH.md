# SPEC: Polimento visual — ícones e tipografia

Status: **proposta, aguardando OK do Helder.** Nenhum código de produto foi alterado. Diagnóstico e capturas: [evidence/ui-polish.md](evidence/ui-polish.md). Tickets: [tickets-ui-polish/](tickets-ui-polish/README.md). Contexto de design: `PRODUCT.md` e `DESIGN.md` na raiz (não alterados). Modo da superfície: **Operate** (escaneabilidade, consistência e expectativa nativa do macOS antes de expressão).

## Problema

Os ícones e as fontes parecem de baixa qualidade. Causas verificadas no app real e no código:

1. **Selo C# é texto CSS de 8 px** (`src/styles.css:2768-2776`), verde saturado; `.csproj`/`.sln` não têm ícone próprio.
2. **Ícones são paths do Lucide copiados à mão e retocados** (`src/Icon.tsx`), com traço de 1,6/24 que renderiza a 1,07 px em 16 px e 1,2 px em 18 px. Play e Debug (e `stop`, `shield`, `solution`, `step*`) saem do estilo do conjunto.
3. **Inter não está embutida.** O DESIGN.md manda Inter; o app renderiza SF Pro (ou Inter, só se instalada). JetBrains Mono é oferecida nas Settings mas não é embutida.
4. **Escala tipográfica solta**: ~15 tamanhos (inclui 0,78 rem, 0,7 rem, 8/9/10 px) para 4 papéis definidos.
5. **Débito de licença**: o Lucide está no código sem aviso em `THIRD-PARTY-NOTICES.md`.

## Princípios aplicados (do PRODUCT.md)

- O código é o destaque; cor fora do neutro significa estado. Ícone de arquivo é neutro (`--muted`).
- Piso de 11 px; alvo de ícone de 32×32; foco por anel; cor nunca é o único portador de estado.
- Zero dependência nova sem decisão registrada: **este spec é essa proposta** e pede aprovação explícita (ver "Dependências").
- `data-testid` e `role="tab"` não mudam.

## Pesquisa e comparação

Licenças confirmadas nos arquivos `LICENSE` upstream e no registro npm em 2026-11-04. "Compatível" = pode ser redistribuído no DMG de um repo MIT, desde que o aviso entre em `THIRD-PARTY-NOTICES.md`.

### Ícones de UI

| Conjunto | Licença | Estilo | Cobertura relevante | Veredito |
|---|---|---|---|---|
| **Lucide** (`lucide-react` 1.68, ~38 MB desempacotado, árvore de ícones importáveis) | ISC (+ MIT para parte herdada do Feather) | traço, grade 24, pontas arredondadas | `play`, `bug`, `bug-play`, `settings`, `hammer`, `square`, `braces`, `file-code`, `folder` existem (verificado). Já é a origem do conjunto atual | **Escolhido**: mesma família do que já está no app, licença permissiva, import por ícone |
| Phosphor (`@phosphor-icons/react` 2.1) | MIT | 6 pesos (thin→fill) | ampla; tem `bug`, `play`, `gear` | Alternativa válida; trocaria toda a identidade sem ganho sobre Lucide. Reserva |
| Tabler (`@tabler/icons-react` 3.50) | MIT | traço, 5000+ ícones | ampla, inclui `brand-c-sharp` (logo) | Alternativa válida; mais ícones do que o necessário, estilo um pouco mais denso |
| Codicons (`@vscode/codicons`) | ícones CC-BY-4.0, código MIT; licença "não concede direito a marcas Microsoft" | glifos pixel-fit de 16 px, estilo VS Code | melhor conjunto de depuração (`debug-start`, `debug-step-*`, …) | Compatível com atribuição, mas traz o visual do VS Code e a cláusula de marcas. Usar só se o ticket 02 reprovar os `step*` do Lucide |
| Material Symbols | Apache-2.0 | variável (peso/preenchimento) | ampla | Estilo "Google"; não é a sobriedade pedida; fonte variável pesa mais |
| MDI / Pictogrammers (`@mdi/js` 7.4) | Apache-2.0 (ícones), MIT (código) | preenchido, grade 24 | `language-csharp` (verificado, 200) | **Usado só para 1 glifo (C#)**, ver abaixo |

### Ícones de arquivo/linguagem

| Conjunto | Licença | Observação | Veredito |
|---|---|---|---|
| Material Icon Theme (`material-icon-theme` 5.40) | repositório MIT; ícones derivados de MDI e Material Symbols | 910 SVGs coloridos. Mapeia `csproj/sln/slnf/slnx` para `visualstudio`, que é o **logo do Visual Studio** (marca Microsoft) | **Rejeitado como conjunto**: colorido (contra o PRODUCT.md) e com logo de marca |
| vscode-icons | código MIT, **ícones CC BY-SA** (share-alike) | share-alike é incompatível com redistribuir sob MIT sem licenciar derivados | **Rejeitado** |
| Seti-ui | MIT (verificado) | conjunto antigo, glifos coloridos | Não avaliado a fundo; sem vantagem frente ao esquema abaixo |
| Devicon / Simple Icons | MIT/CC0 | trazem o logo do C# (hexágono) | Rejeitados por marca (ver abaixo) |

### Marca registrada (C#/.NET)

- O hexágono "C#" e o logo ".NET" são marcas da Microsoft. Não os usamos. O nome "C#" aparece como texto descritivo, o que é uso nominativo.
- O glifo `language-csharp` do MDI é a grafia "C#" em traço próprio, sem hexágono, sob Apache-2.0. É a alternativa segura. Parecer jurídico não foi buscado; a regra do spec é **não usar o hexágono nem o logo do VS**.
- `.csproj`/`.sln` não usam o logo do VS: usam ícones genéricos do Lucide (tabela abaixo).

### Fontes

| Fonte | Licença | Formato / peso | Papel | Veredito |
|---|---|---|---|---|
| **Inter Variable** (`@fontsource-variable/inter` 5.3) | OFL-1.1 | woff2 variável: `latin` 48 KB (cobre pt-BR), `latin-ext` 85 KB | UI | **Escolhida**: é a pinada no DESIGN.md; embutir fecha a divergência código ↔ design |
| **JetBrains Mono Variable** (`@fontsource-variable/jetbrains-mono` 5.3) | OFL-1.1 | woff2 variável: roman 40 KB, itálico 43 KB (latin) | editor e terminal | **Escolhida**: legível em 13 px, itálico verdadeiro para comentários, familiar ao público Rider; já está nas opções das Settings |
| Geist / Geist Mono (`geist`, `@fontsource/geist-mono`) | OFL | — | UI/código | Alternativa; contraria o DESIGN.md sem necessidade. Reserva |
| SF Pro / SF Mono (sistema) | licença Apple, **não redistribuível** | — | — | Não pode ir no DMG; permanece só como fallback |

Obs.: OFL permite embutir e redistribuir com o texto da licença junto, sem vender a fonte isolada. Os woff2 entram no bundle do Vite (offline, sem CDN).

## Combinação recomendada

| Camada | Escolha | Licença |
|---|---|---|
| Ícones de UI e de árvore | **Lucide** via `lucide-react` (import por ícone) | ISC (+ MIT Feather) |
| Glifo C# | **MDI `language-csharp`** (1 path vendorizado em `src/icons/`) | Apache-2.0 |
| UI | **Inter Variable** (latin; latin-ext só se pt-BR/outros precisarem de algo fora do Latin-1) | OFL-1.1 |
| Editor/terminal | **JetBrains Mono Variable** (roman + itálico, latin) | OFL-1.1 |

Por quê: uma família de traço só, a que o app já usa, reduz o risco e a mudança de identidade; todas as licenças são permissivas e cabem no `THIRD-PARTY-NOTICES.md`; as fontes já são as que o DESIGN.md e as Settings citam. Custo estimado de fontes: ~130 KB de woff2 (a medir no ticket 05). Custo de ícones: tree-shaking por import nomeado, ~0,5–1 KB por ícone (a medir no ticket 01; não medido agora).

### Dependências (precisam de aprovação explícita — AGENTS.md)

- `lucide-react` (runtime, ISC)
- `@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono` (runtime, OFL-1.1)
- Alternativa sem dependências de ícone: vendorizar só os SVGs usados de `lucide-static` em `src/icons/` com o `LICENSE`. Troca-se atualização automática por zero pacote. Default do spec: **`lucide-react`**.

## Antes → depois

| Elemento | Antes | Depois |
|---|---|---|
| Selo C# (árvore, aba) | caixa verde com texto 8 px | glifo C# em `--muted` (`--fg` na linha selecionada), 16 px, sem fundo |
| Projeto vs. arquivo `.cs` | mesmo selo | projeto C# e `.cs` usam o glifo C#; a diferença vem da indentação e do rótulo "library · net8.0" (sem segundo ícone) |
| `.csproj`, `.sln` | `file` genérico / ad hoc | Lucide `box` / `blocks` (neutros) |
| `.json` | `{}` roxo texto 12 px | Lucide `braces`, `--muted` |
| Play / Debug | triângulo vivo e inseto fino, verdes | Lucide `play` (preenchido, ver regra de estado) e `bug-play` ou `bug`, verdes mantidos |
| Settings | engrenagem fina | Lucide `settings` em 18 px, traço 1,5 px renderizado, alvo 32×32 |
| Peso de traço | 1,6/24 escalando (1,07–1,2 px) | `absoluteStrokeWidth`: 1,5 px renderizado em qualquer tamanho |
| Alvo de ícone | 22×22 na toolbar da árvore | 32×32 com ícone de 16 px |
| UI | sans do sistema (Inter só se instalada) | Inter Variable embutida |
| Editor | `ui-monospace` | JetBrains Mono Variable embutida, ligaduras desligadas |
| Escala | ~15 tamanhos | 4 tokens: 14 / 13 / 12 / 11 px |

## Lista de ícones a trocar

38 nomes de `IconName` mantêm-se (a API `<Icon name="…"/>` não muda). Mapeamento proposto; **nomes marcados † ainda não verificados no pacote instalado e são conferidos no ticket 01**.

| `IconName` | Lucide |  | `IconName` | Lucide |
|---|---|---|---|---|
| build | `hammer` | | run | `play` |
| debug | `bug` | | stop | `square` |
| file | `file` | | project | `box` † |
| save | `save` † | | code | `code-xml` † |
| rename | `pencil` † | | problems | `triangle-alert` † |
| terminal | `square-terminal` † | | tests | `flask-conical` † |
| search | `search` † | | settings | `settings` |
| git | `git-branch` † | | package | `package` † |
| runconfig | `sliders-horizontal` † | | back / forward | `chevron-left` / `chevron-right` † |
| chevron | `chevron-down` † | | plus / close | `plus` / `x` † |
| locate | `locate-fixed` † | | expandAll / collapseAll | `chevrons-down` / `chevrons-up` † |
| shield | `shield` † | | resume | `play`+barra: `step-forward`/`skip-forward` † |
| stepOver / stepInto / stepOut | **a decidir no ticket 02** (Lucide não tem equivalente canônico; candidatos `redo-2`, `arrow-down-to-line`, `arrow-up-from-line`; fallback Codicons `debug-step-*`) | | pause | `pause` † |
| restart | `rotate-ccw` † | | folder | `folder` |
| solution | `blocks` † | | solutionFolder | `folder-tree` † |
| dependencies | `network` † | | eye | `eye` † |
| more | `ellipsis` / `ellipsis-vertical` † | | minimize | `minus` † |

Arquivos (em `FileIcon.tsx`): `.cs`/projeto C# → glifo C# (MDI); `.csproj`/`.props`/`.targets` → `box`; `.sln`/`.slnx` → `blocks`; `.json` → `braces`; `.md`/`.txt` → `file-text` †; demais → `file`. O Terminal deixa de reutilizar `run` para "Start shell" e passa a `square-terminal` (corrige o uso semântico errado em `TerminalPanel.tsx:226`).

Regras de estado e cor: Play e Debug seguem verdes (convenção do Rider; é estado "pronto para executar") e Stop vermelho; todos os demais ícones neutros. `stop` e `play` são preenchidos (`fill: currentColor`) por serem ações primárias; o resto é traço.

## Lista de fontes a trocar

| Token | Antes | Depois |
|---|---|---|
| `--sans` | `"Inter Variable", Inter, -apple-system, …` (sem embutir) | `"Inter Variable"` embutida, fallback `-apple-system, system-ui, sans-serif` |
| `--mono` | `"SF Mono", ui-monospace, …` | `"JetBrains Mono Variable"` embutida, fallback `ui-monospace, Menlo, monospace` |
| editor default | `ui-monospace` | `JetBrains Mono` (usuários com valor salvo mantêm o que escolheram) |
| terminal | `Menlo, Monaco, 'Courier New'` | mesmo `--mono` do editor |
| escala | ~15 valores, 8–10 px e rem fracionário | `--fs-ui:14px`, `--fs-label:13px`, `--fs-meta:12px`, `--fs-badge:11px`; nenhum valor abaixo de 11 px |
| números | — | `font-variant-numeric: tabular-nums` na status bar e contadores |
| ligaduras | indefinido | `font-variant-ligatures: none` no editor e terminal |
| itálico de comentário | itálico possivelmente sintético | itálico real da JetBrains Mono Variable |

Peso/compensação no escuro (typeset): corpo mantém 400; linha de árvore/aba/botão 500; título 600; avaliar `-webkit-font-smoothing: auto` no texto da UI se o `antialiased` continuar afinando o texto claro (decisão por captura antes/depois no ticket 04).

## Critérios de aceite visuais

Cada item é verificado com `screencapture -x` do **app Tauri real** e crop em resolução nativa, comparado com as capturas "antes" de `docs/evidence/ui-polish/`.

1. Nenhum texto CSS faz papel de ícone (`grep -n "cs-badge\|json-badge" src` vazio).
2. Todos os ícones vêm de um único conjunto (Lucide) mais 1 glifo C#; nenhum path à mão em `src/Icon.tsx`.
3. Traço renderizado de 1,5 px (medido no crop) em 16 e 18 px.
4. Play, Debug, Settings, C#, Build e Stop reconhecíveis a 100% de zoom, sem tocar nas bordas do alvo; alvo de clique ≥ 32×32 (inspecionado no DOM).
5. Árvore: projeto C#, `.cs`, `.csproj`, `.sln`, `.json` e pasta distinguíveis sem ler o nome; nenhum ícone colorido fora Play/Debug/Stop e estados (erro, aviso, sucesso).
6. `document.fonts` mostra Inter Variable e JetBrains Mono Variable **carregadas** a partir de assets do app (não do sistema), com a rede desligada.
7. Nenhum `font-size` abaixo de 11 px nem em rem fracionário em `src/*.css`; os 4 tokens cobrem todos os usos.
8. Comentários do editor em itálico real; `=>`, `!=`, `==` sem ligadura.
9. WCAG AA: ícones de ação ≥ 3:1 sobre o fundo, texto ≥ 4,5:1; foco por anel visível em cada botão de ícone.
10. `bun run check` e `bun run build` passam; e2e: nenhuma falha nova além das conhecidas do AGENTS.md; `data-testid`/`role="tab"` intactos.
11. `THIRD-PARTY-NOTICES.md` lista Lucide (ISC/MIT Feather), MDI `language-csharp` (Apache-2.0), Inter e JetBrains Mono (OFL-1.1) com os textos de licença, e vai para `Contents/Resources` como o atual.
12. O que não puder ser capturado (ex.: Settings, sessão de debug ativa) entra como "não verificado" em `implementation-notes.md`.

## Fora de escopo

- Ícone do app (`src-tauri/icons/`), tema claro, Windows/Intel, redesenho de layout ou paleta.
- Tema de ícones por extensão além de C#/.NET/JSON/Markdown.
- Setting de ligaduras na UI (default: desligadas).

## Decisões e perguntas em aberto

Esta execução foi headless e não pôde perguntar; estas são as premissas assumidas (substituem o `/grill-with-docs`). Cada uma é reversível e deve ser registrada em `decisions.md` ao implementar.

| # | Decisão assumida | Alternativa | Por que o default |
|---|---|---|---|
| D1 | Lucide como único conjunto de UI | Phosphor, Tabler | já é a origem do conjunto; menor mudança de identidade |
| D2 | C# = glifo MDI neutro; sem hexágono nem logo VS | logo oficial | marca registrada; PRODUCT.md rejeita ícone colorido |
| D3 | Ícones de arquivo neutros (`--muted`), não coloridos por tipo | cor por linguagem (estilo Rider/Material) | PRODUCT.md: cor só para estado |
| D4 | Play e Debug continuam verdes | neutros | convenção Rider; **pergunta ao Helder** se quer neutros |
| D5 | Inter embutida, não SF | SF do sistema (mais nativo) | DESIGN.md pina Inter; embutir remove a variação entre máquinas. Trocar é só `--sans` |
| D6 | JetBrains Mono padrão do editor, ligaduras off | manter `ui-monospace`; ligaduras on | familiar ao público Rider; ligaduras off por sobriedade. **Pergunta**: ligaduras on? |
| D7 | `lucide-react` (dependência nova) | vendorizar SVGs | atualização e tree-shaking; **exige aprovação** |
| D8 | `step*` (depuração) pode usar Codicons se Lucide não servir | só Lucide | evita desenhar ícone; mistura limitada e documentada |
| D9 | Alvo 32×32 também nas toolbars compactas da árvore | manter 22 px | PRODUCT.md (Accessibility); pode aumentar a altura do cabeçalho |
| D10 | Valores salvos de fonte do usuário não são migrados | forçar novo default | não mexer em preferência existente |
| D11 | Sem consultoria jurídica sobre marcas; regra conservadora (nada de logo MS) | — | não há como verificar aqui |

Perguntas abertas: D4, D6, D7 e se o cabeçalho de 52 px pode crescer para acomodar D9.

## Plano de verificação

Como no AGENTS.md: checagem objetiva do diff contra este spec e os tickets, depois o skill `verify-openridr` (app Tauri real, capturas antes/depois em `docs/evidence/ui-polish/`). O que o ambiente não permitir vira "não verificado".
