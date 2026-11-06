# SPEC — Primeira abertura com quarentena (Gatekeeper)

Status: ready-for-agent
Data: 2026-11-06. Alvo medido: macOS 27.2 (build 26B118), Apple Silicon. Windows e Intel continuam fora.
Issue-mãe: #45. Retro: `docs/RETRO-FIRST-OPEN.md`. Diagnóstico anterior: #42 (`docs/implementation-notes.md`, "Primeira abertura e verificação embutida").

## Problem Statement

Quem baixa o DMG do OpenRidr e abre o app pela primeira vez vê a interface demorar e, às vezes, o cursor de espera do macOS. O `sample` de uma sessão anterior mostrou o processo parado em `_dyld_start` por 15–18 s, e a conclusão registrada foi "o Gatekeeper está verificando 18 Mach-O assinados antes do nosso código rodar" — o que sugeria um custo no bundle que valeria a pena otimizar.

A medição desta frente desmente essa leitura e mostra dois problemas diferentes, nenhum deles no nosso binário:

1. Na primeira abertura de um app baixado, o macOS apresenta um diálogo do sistema ("… é um app baixado da internet. Tem certeza de que deseja abrí-lo?") e **não inicia o app até alguém clicar** (*Abrir*/Enter). O tempo parado em `_dyld_start` é a espera desse clique, não verificação de assinatura: o diálogo aparece em ~0,4 s e o nosso código só roda depois da decisão.
2. Abrir o app **direto do DMG montado** (em vez de arrastá-lo para Aplicações) custa caro: o sistema lê/copia 164 MB de uma imagem UDZO e o intervalo clique→janela vai para ~5–7 s, contra ~1,4 s quando o app já está no disco.

Sem números separados, a próxima pessoa que investigar "primeira abertura lenta" (ou o próprio Helder, reabrindo a issue) vai continuar atribuindo o atraso ao bundle e propor otimizações de assinatura que não mudam nada.

## Solution

1. **Medir e documentar as fases, não o total.** A primeira abertura passa a ter quatro marcos medidos: `exec` (processo criado), `prompt` (diálogo do Gatekeeper na tela), `code` (o nosso código começa a rodar) e `window` (janela do OpenRidr visível). O número que importa para o produto é `prompt` + `click→window`; a espera humana entre os dois não é atributo do app.
2. **Deixar a medição reproduzível.** Um script único (`scripts/measure-first-open.sh`) monta a condição (cópia nova, quarentena nova, `OPENRIDR_HOME` isolado), dispara a abertura, cronometra os quatro marcos e imprime uma linha por rodada. Sem ele, o próximo diagnóstico repete a leitura errada.
3. **Mitigar por documentação, não por código.** A primeira abertura de um app baixado mostra o diálogo do sistema — é o preço de distribuir fora da App Store, e não há build que o evite (medido nas condições abaixo; tela cheia em outro Space não foi testada). O que dá para dizer ao usuário é o que fazer: **arraste o app para Aplicações antes de abrir** e, na primeira abertura, **clique em Abrir** (ou tecle Enter). O layout de instalação já está correto (o DMG publicado tem o atalho `Applications -> /Applications` ao lado do app), então não há mudança de empacotamento nesta frente.
4. **Registrar o que não se reduz.** A conclusão "não redutível pelo bundle" vai para `docs/evidence/first-open.md` com a tabela de rodadas, para `docs/decisions.md` e para as notas de release, junto com o que ficou sem medição (tela cheia em outro Space, macOS 26, Intel, Windows).

## User Stories

1. Como pessoa que baixou o DMG e abriu o OpenRidr pela primeira vez, quero entender por que nada acontece por alguns segundos, para não concluir que o app travou.
2. Como pessoa na primeira abertura, quero saber que basta clicar em *Abrir* (ou teclar Enter) no diálogo do macOS, para chegar à janela em ~1,5 s em vez de esperar parado.
3. Como pessoa que abre o app direto da janela do DMG, quero ser avisada de que arrastar para Aplicações antes de abrir é mais rápido.
4. Como pessoa que arrastou o app para Aplicações, quero que a primeira abertura continue mostrando o diálogo do sistema e que, depois disso, abrir o app seja direto.
5. Como pessoa que já abriu o app uma vez, quero que reabrir não mostre mais o diálogo nem pague o custo da primeira abertura.
6. Como mantenedor, quero a primeira abertura decomposta em `exec`, `prompt`, `code` e `window`, para não confundir espera do sistema com lentidão do produto.
7. Como mantenedor, quero um número de referência para "app notarizado e grampeado, cópia local": prompt em ~0,4 s e janela em ~1,4 s depois do clique.
8. Como mantenedor, quero um número de referência para "aberto direto da imagem": prompt em ~1,5 s e janela em ~5–7 s depois do clique, para reconhecer esse caminho quando ele aparecer de novo.
9. Como mantenedor, quero um número de referência para "sem quarentena" (~1,0 s até a janela), que isola o nosso próprio arranque.
10. Como mantenedor, quero saber que o ticket grampeado não é gargalo: remover `Contents/CodeResources` (ticket online) não mudou o tempo do diálogo.
11. Como mantenedor, quero saber o que **não** está sendo pago: o número de Mach-O (18) e o layout em `Contents/Resources` não aparecem como custo mensurável antes do diálogo.
12. Como mantenedor, quero um script que reproduza a medição sem eu lembrar a receita de cópia, quarentena, `launchctl setenv` e clique.
13. Como mantenedor, quero que o script recuse rodar se já existir uma instância do app, para não medir o plugin de instância única nem atrapalhar outra sessão.
14. Como mantenedor, quero uma linha por rodada e a contagem de quantas chegaram à janela, para ver a dispersão em vez de confiar num número solitário.
15. Como mantenedor, quero que o script documente como invalidar o cache entre rodadas, porque repetir na mesma cópia/caminho deixa de mostrar o diálogo.
16. Como mantenedor, quero que as notas de release da próxima alpha expliquem o diálogo de primeira abertura, para reduzir relato de "travou".
17. Como pessoa que lê `docs/install-app.md`, quero a instrução de arrastar para Aplicações antes de abrir, com o motivo e o número.
18. Como pessoa que lê a evidência, quero ver a tabela de rodadas com data, máquina, versão do app e carga do sistema, para saber o quanto confiar nos números.
19. Como revisor, quero que a evidência declare explicitamente o que não foi medido (outro Space/tela cheia, outro macOS, nova notarização, Mac sem cache), em vez de generalizar.
20. Como revisor, quero que a decisão "não redutível pelo bundle" esteja em `docs/decisions.md`, para que a pergunta não volte como se nada tivesse sido medido.
21. Como pessoa que abre a issue #45, quero a conclusão e o critério de pronto atendidos: medição feita, conclusão clara (não redutível) e mitigações aplicadas.
22. Como mantenedor, quero que a issue #45 feche sem prometer redução que não existe.

## Implementation Decisions

- **Precondição de assinatura.** O script recusa um bundle que não seja `Developer ID` e aceito pelo Gatekeeper (`--allow-unnotarized` para medir outro), porque o default `/Applications/OpenRidr.app` também é o destino do `bun run app:install` ad-hoc, e medir esse build descreveria outro diálogo.
- **Marcos e definições** (cinco, mais dois deltas). `exec` = processo do app aparece; `prompt` = linha `present code-evaluation prompt` do `CoreServicesUIAgent` no log unificado; `code` = RSS do processo passa de 20 MB (o processo fica suspenso pela quarentena até a decisão); `window` = janela do OpenRidr listada como visível. O relógio de cada rodada começa imediatamente antes da chamada de abertura.
- **Condições medidas** (todas com o app alpha.11 do release publicado, sem builds novos):
  - **E0 sem quarentena** — cópia em diretório gravável, xattr removido: isola o nosso arranque.
  - **E1 quarentena, cópia em disco** (`0083` + UUID novo, caminho novo): o caminho de quem arrastou para Aplicações; o app roda translocado (`AppTranslocation` no path do processo).
  - **E3 cópia de dentro do DMG montado** (a cópia herda `00c3` do volume): equivalente a arrastar do DMG para o disco.
  - **E4 sem `Contents/CodeResources`** (ticket online, mesma assinatura): separa "grampeado" de "online".
  - **E3m/E3c abrir direto da imagem** (app dentro do DMG, sem cópia): o caminho lento.
- **Regra de corte.** As variantes de bundle (runtime em `tar.gz` para 1 Mach-O, runtime em `Contents/Frameworks`, bundle sem runtime) testariam o custo de avaliação, e a condição para isso é o app **no disco**. Ali nenhum marco passou de 0,53 s até o diálogo e 0,24 s depois do clique, então as variantes ficam fora — não haveria o que economizar que coubesse no limite de ~2 s combinado. Nas condições que abrem de dentro da imagem (E3m/E3c) `click→code` passou de 2 s; leitura da imagem é a explicação compatível (`ditto` 1,95 s, prompt em 1,38–1,51 s) mas o split não foi isolado; as variantes de layout não mudariam esse número e as que encolhem o payload são outra frente (fora de escopo).
- **Cache e aprovação do sistema.** Medir de novo a mesma cópia no mesmo caminho **não** repete o diálogo: o macOS memoriza a aprovação. Cada rodada usa caminho novo + xattr de quarentena novo (UUID novo). O cache do `syspolicyd` não pode ser limpo nesta máquina (exige root); "primeira avaliação de um cdhash nunca visto" só é medível com build novo — e esta frente **não** faz notarização nova (decisão do Helder), então isso entra como limitação declarada.
- **Clique.** O diálogo do `CoreServicesUIAgent` não expõe superfícies de acessibilidade (`list-surfaces` vazio), então a dispensa é clique físico via `agent-desktop` nas coordenadas do botão *Abrir*, com uma captura de tela por rodada no modo `--click` como prova. O script não deve tentar "Enter" às cegas: quando o diálogo está focado, Enter equivale a *Abrir* (verificado), mas sem foco o Enter vai para o app da frente.
- **Instância única.** O script aborta se houver `openridr` rodando; abrir duas vezes cai no plugin de instância única e mede outra coisa.
- **Identidade do processo e do diálogo.** Cada rodada lança a cópia com um marcador no argv (`--openridr-measure-round=N`, ignorado pelo app) e só reconhece como seu um processo que não existia antes e carrega esse marcador; dois candidatos com o marcador viram falha de rodada (identidade ambígua), não escolha. O alerta do Gatekeeper é global (do `CoreServicesUIAgent`), então só conta depois que o processo da rodada existe — e o clique que não iniciar o app em 5 s falha a rodada em vez de virar número. A janela só conta se for `OpenRidr`, `visible` e do pid medido.
- **Isolamento de estado.** `OPENRIDR_HOME` aponta para um diretório temporário via `launchctl setenv` (o `open` não propaga variáveis de ambiente), com restauração no fim; o app abre na Welcome, sem tocar no estado real do usuário.
- **Nada de `/Applications`.** As cópias de medição vivem em diretório temporário; a instalação do usuário não é substituída, e nenhum release/tag/push é produzido.
- **Captura e falha.** No modo `--click`, cada rodada tira a captura do alerta antes de clicar; se a captura falhar, a rodada falha (não clica às cegas).
- **Onde cada coisa mora.** Script em `scripts/`; números e limitações em `docs/evidence/first-open.md`; decisão em `docs/decisions.md`; instrução de uso em `docs/install-app.md`; parágrafo para as notas em `docs/RELEASE-ALPHA.md`; resumo do que foi feito e do que não foi verificado em `docs/implementation-notes.md`.
- **Layout do DMG.** O DMG publicado já traz `Applications -> /Applications` ao lado do app, então a mitigação de empacotamento está satisfeita e não gera ticket. Se o pipeline da #44 (issue #66) mudar a montagem do DMG, o layout precisa continuar assim; esse acoplamento fica registrado no spec, não como bloqueio.

## Testing Decisions

- **O script é o teste.** O comportamento sob teste é do sistema operacional, não do nosso código: não há seam de unidade aqui. A verificação é rodar `scripts/measure-first-open.sh` de verdade e conferir que ele (a) recusa rodar com o app aberto, (b) imprime os quatro marcos por rodada, (c) deixa o `OPENRIDR_HOME` restaurado e nenhum processo órfão.
- **Falha fechado.** Sem `agent-desktop` (ou sem permissão de acessibilidade, e de gravação de tela no modo `--click`), o script para com mensagem clara em vez de esperar para sempre; se o app não arrancar 5 s depois do clique, se a janela aparecer sem o marco `code`, ou se `open`/`ditto` falharem, ele marca a rodada como falha em vez de inventar um número. Só um processo que apareceu depois do nosso `open` é medido e encerrado.
- **Regressão de documentação.** A tabela da evidência cita versão do app, sha256 do DMG, data, máquina e carga média; qualquer re-medição futura entra como linha nova, não substitui a antiga.
- **Prior art.** O padrão de "prova real + o que não foi verificado" é o de `docs/evidence/*.md` e da skill `verify-openridr` (app real via `agent-desktop`, nada de simulação).

## Out of Scope

- Reduzir o número de Mach-O, mudar layout do bundle (`Resources` → `Frameworks`), trocar formato de DMG (zip, APFS sem compressão) ou reduzir o payload embutido. Há números que sugerem que o formato da imagem é o que dói no caminho "abrir direto do DMG", mas isso é outra frente (e depende de medir a partir de imagem fria sem root).
- Notarizações novas, releases, tags, push, PR, substituição do app instalado.
- Pipeline de release/CI (#44) e a auditoria de assinatura (#65).
- Tela cheia em outro Space (o diálogo pode não aparecer; não medido), macOS 26, macOS Intel, Windows.
- Qualquer mudança de código de produto: o nosso arranque já foi otimizado na #42 e não é o gargalo.
- Atualização automática (#55) e releases multi-plataforma (#54).

## Further Notes

Medição de 2026-11-06, app `0.3.0-alpha.11` (DMG do release, sha256 `15bf37e8…0510`), carga média 7–9 (outras sessões ativas), `OPENRIDR_HOME` isolado:

| condição | marcos (s) | clique→código | clique→janela |
|---|---|---|---|
| E0 sem quarentena | exec 0,06–0,12 · code 0,13–0,55 | — | janela em 1,00 |
| E1 quarentena, cópia em disco (3 rodadas) | exec 0,19–0,28 · prompt 0,40–0,53 | 0,14–0,24 | 0,97–1,42 |
| E4 sem ticket grampeado (2 rodadas) | prompt 0,32–0,38 | 0,21–0,22 | 1,36–1,60 |
| E3 cópia de dentro do DMG (2 rodadas) | prompt 0,26–0,35 | 0,18–0,21 | 0,54–0,58 |
| E3m abrir direto do DMG (1ª vez) | prompt 1,38 | 4,43 | 5,12 |
| E3c abrir direto de imagem fria (1 rodada) | prompt 1,51 | 5,44 | 6,57 |

O "18 s" original é a soma de ~1,5 s até o diálogo + tempo de reação humana + ~4–5 s de leitura da imagem quando o app é aberto de dentro do DMG; com o app copiado para o disco, a janela aparece ~1,4 s depois do clique. `ditto` do mesmo app a partir da imagem montada leva 1,95 s, o que dimensiona a leitura. Como o limite superior do trabalho do sistema antes do diálogo é ~0,5 s (app no disco), nenhuma mudança de assinatura, layout ou contagem de Mach-O poderia devolver mais que isso.
