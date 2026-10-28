# Decisão de release P0 — OpenRidr

Data: 28/10/2026. Base: `4b8885c` + commit `feat(acceptance)`.
Referências: [matriz de aceitação](acceptance-matrix.md), [evidência 18](evidence/18-aceitacao-e-release.md),
[SPEC](SPEC.md), [índice de tickets](tickets/README.md).

## Decisão

**NÃO APROVADO — RELEASE BLOQUEADO.**

Esta entrega consolida evidência real do núcleo P0 em macOS ARM64 e registra, sem arredondar,
o que falta para publicar. Ela **não** declara P0 aceito, **não** libera distribuição e **não**
autoriza publicar apenas o Mac como “entrega reduzida”. Mac e Windows continuam juntos no P0.
Nenhuma evidência de desenho/documentação é usada como prova de execução.

## O que está aprovado (somente o que foi executado)

- **macOS ARM64, núcleo P0 real**: trust/revogação, SDK host 10.0.100 separado do projeto
  8.0.408, avaliação MSBuild, restore explícito, csharp-ls 0.28.0 com completion/definição/
  rename/code actions/diagnósticos, build falho→corrigido→Run, VSTest (descoberta, selected,
  all, cancelamento) e PTY zsh com Ctrl+C. Evidência: E01–E17 + A18, `L18-*`, `S18-screen-01…07`.
- **Debug experimental em macOS ARM64**: netcoredbg 3.2.0-1092 osx-arm64 com opt-in explícito,
  launch/breakpoint verificado/pausa/F8/locals/continue exit 0/stop observado. Isso aprova o
  **caminho experimental**, não um suporte de release; a flag continua OFF por padrão.
- **Auditoria e decisão rastreáveis**: matriz AC-01…AC-34 × 3 ambientes, G1–G5, auditoria de
  UI/acessibilidade heurística, auditoria de componentes e esta decisão.
- **Correções pequenas encontradas na auditoria** (sem novo contrato de host): Run macOS passou
  a responder a `Ctrl+R` (antes o handler usava Cmd+R, divergindo da SPEC e do rótulo); painel
  inferior reseta rolagem ao trocar de aba; saída/avisos podem encolher em 1100×700; o terminal
  pode rolar o próprio painel em vez de esconder shell/cwd; o rótulo LSP não duplica “partial
  coverage”; processo LSP sem exit mostra “exit not observed”, não “exit unknown”.

## O que está bloqueado

| Gate | Por que bloqueia | Critério de saída |
|---|---|---|
| **G-WIN-CORE** | Nenhum núcleo P0 foi executado em Windows x64: UI desktop, trust, SDKs, avaliação, restore, LSP, build/run, VSTest, PTY e término de processos Windows. CI local não existe; CI só de CLI/contratos não prova a UI. | Jornada real na UI Windows x64 (não só CLI), com logs, comandos, exits, processos remanescentes e screenshots; CRLF/paths/permissões cobertos. |
| **G-WIN-DAP** | O asset `netcoredbg-win64.zip` existe no metadata, mas não foi instalado nem executado. A presença do binário não prova DAP. | Smoke Windows x64 com `--interpreter=vscode`: launch, breakpoint pendente→verificado, pausa, step over/into, frames, locals, continue/stop, logs e processos encerrados. |
| **G-INTEL** | macOS Intel não foi executado; o núcleo P0 da matriz inclui Intel. Não há asset `osx-x64` do netcoredbg 3.2, então Debug fica indisponível sem substituição — mas o núcleo (trust/SDK/árvore/LSP/build/run/testes/PTY) continua obrigatório. | Núcleo real em macOS Intel, ou reescopo explícito aprovado pelo usuário; não reduzir a entrega a “Mac ARM64”. Debug Intel permanece indisponível/documentado se não houver asset compatível. |
| **G-LEGAL** | Não existe `LICENSE`, `NOTICE` ou `THIRD_PARTY_NOTICES` na raiz; o pacote declara Apache-2.0, mas o texto não é distribuído. Dependências MIT/Apache/BSD/Unicode e cinco crates MPL-2.0 (cssparser, cssparser-macros, dtoa-short, option-ext, selectors) exigem avisos; csharp-ls e netcoredbg são MIT e baixados pelo app; o único ícone é o default do Tauri, não uma identidade OpenRidr documentada. | Licença do projeto, avisos de terceiros (incluindo MPL e MIT de csharp-ls/netcoredbg), ícone/identidade própria licenciada, revisão jurídica e manifestos incluídos no instalador. |
| **G-PKG** | `bundle.active` é `false`; não há ícones de bundle, assinatura/notarização macOS, assinatura Windows nem smoke de instalador. | Bundle real Mac+Windows a partir do mesmo commit, ícones completos, assinatura/notarização aplicáveis, instalador testado, avisos/licenças incluídos. |
| **G-NATIVE** | A UI foi dirigida no Chrome real contra Vite + host-http (mesmo `dispatch`); a janela Tauri nativa não foi automatizada/capturada por TCC no macOS. | Smoke manual ou automatizado da janela nativa macOS (e Windows no gate Windows): foco, atalhos, fechamento com buffers, diálogos, sem erros de console. |
| **G-DISCLOSE** | Limitações conhecidas de verificação: NuGet imprime “Skipping NuGet package signature verification”; SHA-256 do netcoredbg vem do digest do GitHub, sem assinatura/notarização; `analyzerCoverage` é `not_verified`; MTP não é suportado; net6 é best effort; #256 é risco não reproduzido. | Release notes e superfície do produto declararem exatamente esses limites; aprovação de produto/segurança/jurídico para o que não for eliminado. Nada disso pode ser apresentado como verificado. |

Além disso, a suíte E2E depende da ordem alfabética para o spec de aceitação rodar por último
(`zz-acceptance.spec.ts`); os testes por ticket assumem contadores/sessões iniciais. Isso é dívida
de teste, não gate de produto, mas precisa ser corrigido ou documentado antes de CI estável.

## O que não foi verificado

- macOS Intel: nenhum AC do núcleo; Debug indisponível por ausência de asset 3.2 `osx-x64`.
- Windows x64: nenhum AC do núcleo, PTY, VSTest ou DAP; teclado Windows só foi inspecionado no código.
- Outras arquiteturas Windows e Linux: fora do P0; sem promessa.
- net6 e outros TFMs .NET 8+: best effort/não executados; a aprovação cobre somente net8.0.
- Analyzers/pull diagnostics; MTP é explicitamente não suportado; TRX não descobre testes.
- Leitor de tela/WCAG formal; a auditoria de nomes/foco/contraste/overflow é heurística.
- Janela Tauri nativa e instaladores assinados (ver gates).
- Qualquer comportamento de debug universal em Macs ou compatibilidade implícita por presença de asset.

## Condições para publicar Mac + Windows juntos

1. Fechar **G-WIN-CORE**, **G-WIN-DAP**, **G-INTEL**, **G-LEGAL**, **G-PKG** e **G-NATIVE** com
   evidência real por ambiente, do mesmo commit.
2. Publicar os artefatos Mac e Windows juntos; **Mac sozinho não é entrega reduzida** e não
   substitui os gates Windows/Intel.
3. Manter a flag ARM64 **OFF por padrão** e o debug como experimental; não retirar a flag sem
   nova prova DAP no build corrente e decisão explícita. #256 continua risco não reproduzido.
4. Declarar no release notes: net8.0 verificado nesta fixture; net6 best effort; outros TFMs
   exigem pré-requisitos/evidência próprios; Windows/Intel só após seus gates; MTP não suportado;
   limites de assinatura/hash/notarização.
5. Incluir licenças/avisos no bundle e confirmar que a ponte HTTP de desenvolvimento
   (`openridr-host-http`) **não** é empacotada (hoje ela não é dependência do app Tauri, mas isso
   deve ser verificado no artefato final).
6. Se qualquer gate acima continuar aberto, a decisão permanece **bloqueada**. Não há aprovação
   condicional, silenciosa ou por ausência de evidência.

## Riscos conhecidos que não podem ser escondidos

- **#256 (netcoredbg)**: risco upstream relatado, não reproduzido aqui; não é falha universal nem
  compatibilidade comprovada.
- **Integridade de ferramentas**: csharp-ls sem verificação de assinatura NuGet; netcoredbg com
  SHA do digest do GitHub e sem assinatura/notarização; o app não inventa hash/assinatura.
- **Identidade**: o ícone atual é o default do Tauri; a marca OpenRidr ainda não tem asset próprio
  documentado/licenciado.
- **Licenças**: cinco crates MPL-2.0 no grafo macOS e avisos MIT/Apache/BSD/Unicode não empacotados.
- **Ordem de testes**: spec de aceitação roda por último para preservar as premissas dos tickets.
- **Debug experimental**: funciona no opt-in desta máquina, mas não é suporte de release nem
  promessa para Intel/Windows.

## Assinatura

Esta decisão é técnica e documental. **Não há aprovação jurídica, de segurança, de produto ou de
release.** Nenhum artefato foi publicado, assinado, notarizado ou enviado a remote/CI.

- Decisão: **BLOCKED — NOT APPROVED**
- Data: 28/10/2026
- Responsável pela consolidação: sessão de implementação do ticket 18 (OpenCode)
- Aprovação humana/legal: **pendente**
