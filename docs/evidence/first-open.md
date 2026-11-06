# Evidência — primeira abertura com quarentena (Gatekeeper)

Data: 2026-11-06. Branch `fix/issue-45-gatekeeper-first-open`. Issue: #45. Spec:
[`../SPEC-FIRST-OPEN.md`](../SPEC-FIRST-OPEN.md). Diagnóstico anterior que esta medição corrige:
`../implementation-notes.md` ("Primeira abertura e verificação embutida") e #42.

## Pergunta

"Por que a primeira abertura depois de baixar o DMG leva ~18 s com `_dyld_start` e cursor de
espera, e isso é redutível pelo nosso bundle?"

## Ambiente

- macOS 27.2 (build 26B118), Apple Silicon; `xcode-select -p` = Command Line Tools (sem Xcode).
- App medido: `0.3.0-alpha.11` do release publicado
  (`gh release download v0.3.0-alpha.11`), DMG `OpenRidr_0.3.0-alpha.11_aarch64.dmg`,
  sha256 `15bf37e8355127796ac0cf03b2c767f31495275ebc9847d623dd60c027880510` (confere com o
  digest do release).
- Bundle: 164 MB, 18 Mach-O: 13 dylibs do runtime .NET 10 + `createdump` + o muxer `dotnet`
  (= 15 em `Resources/dotnet`), mais `netcoredbg`, `libdbgshim.dylib` e `openridr`. 423 arquivos no
  total, 192 na árvore `Resources/dotnet`.
- Assinatura: `Developer ID Application: Helder Santos (K6M85697D4)`, hardened runtime,
  notarizado e grampeado (`spctl --assess --type execute` = *accepted, source=Notarized
  Developer ID*; `stapler validate` = ok).
- Carga média durante as rodadas: 7–9 (outras sessões de agente ativas no mesmo Mac). Os números
  são de uma máquina ocupada, não de um benchmark isolado; a separação de fases não depende disso.
- Nenhum build novo, nenhuma notarização nova, `/Applications` intocado: as cópias de medição
  vivem em `$TMPDIR`. `OPENRIDR_HOME` isolado por `launchctl setenv` (o `open` não propaga
  ambiente) e restaurado no fim.

## Método

Harness em `scripts/measure-first-open.sh` (a medição abaixo foi feita com o protótipo que virou
esse script). Quatro marcos por rodada, com o relógio começando imediatamente antes do `open`:

| marco | como é detectado |
|---|---|
| `exec` | `pgrep -x openridr` passa a achar o processo |
| `prompt` | linha `present code-evaluation prompt` do `CoreServicesUIAgent` no log unificado |
| `code` | RSS do processo passa de 20 MB (a quarentena deixa o processo suspenso até a decisão) |
| `window` | `agent-desktop list-windows --app OpenRidr` lista a janela como visível |

Dispensa do diálogo: clique físico em *Abrir* via `agent-desktop mouse-click` (o
`CoreServicesUIAgent` expõe **zero** superfícies de acessibilidade: `list-surfaces` devolve
`{"surfaces":[]}`, como o `../release-signing.md` já registrava).

Resolução: cada iteração do harness custa ~0,1–0,2 s (`pgrep`, `ps`, `perl`, `log`, `agent-desktop
list-windows`) e cada marco é carimbado **quando a observação chega**, ou seja, até um período de
amostragem *depois* do evento real. Os centésimos tabelados estão atrasados nessa ordem de
grandeza (não é microbenchmark); por isso as conclusões usam faixas e ordem de grandeza, nunca
comparações de 10 ms.

### Invalidar entre rodadas

- Aprovação é **por caminho**: reabrir a mesma cópia no mesmo diretório não mostra diálogo
  (medido: 2ª e 3ª abertura do app dentro do mesmo DMG montado arrancaram em 0,19/0,22 s, sem
  diálogo). Cada rodada usa diretório novo + xattr novo.
- Xattr de quarentena: `0083;<epoch em hex>;Safari;<uuid novo>`.
- O cache do `syspolicyd` **não** pôde ser limpo: `sudo` pede senha nesta sessão (sem root).
  Portanto "primeira avaliação de um cdhash nunca visto" não foi medido; todas as rodadas usam o
  cdhash do alpha.11, que o sistema potencialmente já conhecia.
- Cópia de dentro de um DMG montado herda quarentena do volume (`00c3;00000000;;<uuid do DMG>`),
  que é o que o Finder faz ao arrastar. Essa é a condição E3.

## Resultados (segundos, desde o `open`)

Uma linha por rodada; deltas calculados dos valores brutos (as colunas arredondam para dois
decimais, então a subtração pode não fechar exatamente).

| condição | rodada | exec | prompt | click | code | window | click→code | click→window |
|---|---|---|---|---|---|---|---|---|
| E0 sem quarentena | 1 (cache de página frio) | 0,12 | — | — | 0,55 | — | — | — |
| E0 sem quarentena | 2 | 0,06 | — | — | 0,13 | 1,00 | — | — |
| E1 quarentena `0083`, cópia em disco | 1 | 0,19 | **0,53** | 1,05 | 1,28 | 2,01 | 0,24 | 0,97 |
| E1 quarentena `0083`, cópia em disco | 2 | 0,28 | 0,42 | 1,21 | 1,35 | 2,63 | 0,14 | 1,42 |
| E1 quarentena `0083`, cópia em disco | 3 | 0,24 | 0,40 | 1,04 | 1,27 | 2,42 | 0,23 | 1,38 |
| E4 sem `Contents/CodeResources` (online) | 1 | 0,16 | 0,32 | 0,76 | 0,97 | 2,35 | 0,21 | 1,60 |
| E4 sem `Contents/CodeResources` (online) | 2 | 0,21 | 0,38 | 2,04 | 2,26 | 3,40 | 0,22 | 1,36 |
| E3 cópia de dentro do DMG montado | 1 | 0,21 | 0,35 | 0,90 | 1,08 | 1,47 | 0,18 | 0,58 |
| E3 cópia de dentro do DMG montado | 2 | 0,14 | 0,26 | 0,67 | 0,88 | 1,22 | 0,21 | 0,54 |
| E3m abrir direto do DMG, 1ª vez no caminho | 1 | 0,19 | 1,38 | 1,78 | 6,21 | 6,90 | 4,43 | 5,12 |
| E3m mesmo caminho, aprovação memorizada | 2 | 0,09 | — | — | 0,19 | 0,63 | — | — |
| E3m mesmo caminho, aprovação memorizada | 3 | 0,07 | — | — | 0,22 | 1,21 | — | — |
| E3c abrir direto de imagem **fria** (DMG recém-copiado) | 1 | 0,21 | 1,51 | 2,27 | 7,71 | 8,85 | 5,44 | 6,57 |

Faixas por condição, para leitura rápida: E1 → prompt 0,40–0,53, clique→código 0,14–0,24,
clique→janela 0,97–1,42; E4 → prompt 0,32–0,38, clique→código 0,21–0,22, clique→janela 1,36–1,60;
E3 → prompt 0,26–0,35, clique→código 0,18–0,21, clique→janela 0,54–0,58.

Referências auxiliares:

- `spctl --assess --type execute -vv` no app grampeado: **0,27 s**; sem `Contents/CodeResources`
  (consulta online): **0,43 s**. A avaliação de assinatura do bundle inteiro é sub-segundo.
- `spctl --assess --type open --context context:primary-signature` no DMG: 0,43 s; montar o DMG
  quarentenado não apresenta diálogo.
- `ditto` do app de dentro do DMG montado para o disco: **1,95 s** (é o custo do arrasto).

## O que o log do sistema mostra

Linhas reais da rodada E1-1 (`open` às 13:21:05.844). Os quatro primeiros valores vêm do
timestamp do log (`log show`), ancorados nesse `open`; os três últimos são o relógio do harness,
que tem resolução de ~0,1 s:

```
+0,17  syspolicyd          GK performScan (team K6M85697D4)      ← avaliação começa
+0,18  CoreServicesUIAgent XProtect finished executable analysis ← malware/signature check pronto
+0,52  CoreServicesUIAgent present code-evaluation prompt        ← diálogo na tela (0,53 no harness)
+1,05  clique em Abrir (agent-desktop)                            ← decisão
+1,07  CoreServicesUIAgent progressed to 0% … 100%                ← quarentena processada
+1,28  nosso código roda (RSS > 20 MB)                            ← clique→código 0,24
+2,01  janela do OpenRidr visível                                 ← clique→janela 0,97
```

E a linha que explica o `_dyld_start` (rodada de protótipo, 13:10:01.244, mesmo app):

```
CoreServicesUIAgent  LAUNCH: 0x0-0x4af4af com.openridr.desktop launched with
                     launchInQuarantine == true, so not starting the application
```

O processo é criado e **fica parado em `_dyld_start` enquanto o diálogo espera resposta** — por
quanto tempo ninguém clicar. Numa rodada sem `--click` em que a resposta demorou, o diálogo ficou
18,4 s na tela e o código só começou 18,5 s depois dele aparecer: é o "18 s" relatado,
reproduzido de propósito. O `sample` que mediu "15–18 s em `_dyld_start`" estava olhando essa
espera, não verificação de assinatura — a avaliação inteira antes do diálogo levou 0,52 s (0,53 no relógio do harness).

## Sobre matar o app no meio

Killing the OpenRidr process does **not** dismiss the alert when the app never started: the alert
belongs to `CoreServicesUIAgent`, so it stayed on screen after a round was interrupted (visto às
13:58). O script passou a mandar `Escape` no fim de uma rodada sem `code` no modo `--click`, e a
avisar o operador no modo manual. Fechar o diálogo com *Cancelar* não inicia o app.

## Provas auxiliares

App translocado (é assim que o macOS roda um app em quarentena fora de `/Applications`):

```
$ ps -o command -p <pid>
/private/var/folders/n4/.../T/AppTranslocation/77F40975-6564-45D8-9513-A3AB52C4AF33/d/OpenRidr.app/Contents/MacOS/openridr
```

Layout do DMG publicado (atalho para Aplicações ao lado do app — a mitigação de empacotamento):

```
$ hdiutil attach OpenRidr_0.3.0-alpha.11_aarch64.dmg -nobrowse -mountpoint mnt4
$ ls -la mnt4
lrwxr-xr-x  Applications -> /Applications
drwxr-xr-x  OpenRidr.app
```

O diálogo do Gatekeeper não expõe acessibilidade (`agent-desktop list-surfaces --app
CoreServicesUIAgent` → `{"surfaces":[]}`), por isso a dispensa é clique físico; o `data` do
clique no app é o path translocado acima, não a cópia original.

## Conclusão

1. **O trabalho do sistema antes do diálogo é sub-segundo.** Com o app notarizado/grampeado no
   disco, o Gatekeeper leva **≤ 0,53 s** até pôr o diálogo na tela (0,17 s até começar a avaliação,
   0,18 s até o XProtect terminar). Tudo o que o bundle poderia economizar está, portanto,
   **limitado a esse teto sub-segundo** — inclusive número de Mach-O e layout. Remover o ticket
   grampeado (E4) não muda a ordem de grandeza: 0,32–0,38 s.
2. **O que o usuário percebe como travamento é o diálogo esperando clique**, e não o app. É
   comportamento do macOS para qualquer app baixado: o processo fica suspenso (`_dyld_start`) até
   alguém responder. Medido: 0,14–0,24 s do clique até o nosso código e 0,97–1,42 s até a janela.
   A conclusão da #45 é essa: o atraso **não** está no bundle (teto de 0,53 s); o que se mediu é o
   clique que o macOS espera e, no caminho do DMG, 1,95 s de cópia + um intervalo clique→código de
   4,4–5,4 s cujo dono não foi isolado (leitura da imagem é a explicação compatível).
3. **O caminho "abrir direto do DMG" é caro e é redutível por instrução, não por código:** clique
   → janela leva **5,1–6,6 s**, contra 0,97–1,42 s com o app copiado para o disco. O que está
   medido: copiar o app de dentro da imagem leva 1,95 s (`ditto`), e ler os 164 MB da imagem
   comprimida é a explicação **compatível** com os 5,1–6,6 s — não foi isolado quanto desse tempo é
   cópia e quanto é o nosso arranque lendo resources da imagem translocada. Arrastar para
   Aplicações antes de abrir evita o caminho inteiro.
4. **Regra de corte aplicada apenas onde ela mede o bundle.** As variantes de bundle (runtime em
   `tar.gz` para 1 Mach-O, runtime em `Contents/Frameworks`, bundle sem runtime) testariam o custo
   de avaliação, então a condição é o app **no disco**: ali nenhum marco passou de 0,53 s até o
   diálogo e 0,24 s depois do clique, muito abaixo do limite de ~2 s combinado — por isso **não
   foram medidas**. Nas condições que abrem de dentro da imagem (E3m/E3c) `click→code` passou de
   2 s; nenhuma medição aponta avaliação de assinatura como causa (o prompt ali aparece em
   1,38–1,51 s, e `ditto` da mesma cópia custa 1,95 s), mas o split entre cópia e arranque não foi
   isolado. As variantes de layout (runtime em `Contents/Frameworks`) não mudariam esse número; as
   que encolhem o payload (runtime em `tar.gz`, sem runtime) atacam tamanho — outra frente, que o
   spec deixa fora de escopo.
5. **Diálogo visível e focável nas condições testadas.** Com Finder e com T3 Code na frente
   (janela maximizada no centro), o diálogo apareceu **na frente e centralizado** em todas as
   rodadas com captura; e quando ele está na tela o *Return* equivale a *Abrir* (app arrancou
   0,25 s depois do Return). Tela cheia em outro Space não foi medida (ver abaixo).

## Não verificado

- **Tela cheia em outro Space**: não medido. Colocar um app em fullscreen via `agent-desktop`
  falhou nesta sessão (`AXMenuItemCmdModifiers` inválido no atalho) e não se insistiu para não
  mexer mais na máquina do Helder. É a hipótese mais provável para "diálogo aparece atrás/em
  outro Space" — precisa de uma janela livre com o Mac disponível.
- **cdhash nunca visto**: não medido (exigiria build novo + notarização; decisão de não fazer
  notarização nesta frente). Todas as rodadas usam um cdhash que o sistema pode já ter avaliado.
  O que se sabe: a avaliação do bundle em si custa sub-segundo (`spctl`), e o diálogo aparece em
  0,4 s mesmo assim.
- **Cursor de espera (beachball)**: capturas via ScreenCaptureKit não desenham o cursor, então não
  há prova visual do cursor. O que existe medido: 0,3–0,5 s entre o `open` e o diálogo sem nada na
  tela, e nenhum estado do processo em que ele esteja bloqueado *sem* o diálogo à vista — o
  processo fica suspenso pela quarentena, e o diálogo é a interface dessa suspensão.
- **macOS 26 e anterior, macOS Intel, Windows**: não executados.
- **E0 (sem quarentena)**: 2 rodadas, mas a primeira só tem `exec`/`code` (sem captura de janela) e
  a segunda reaproveita o cache de página da cópia; **E3c (imagem fria)**: 1 rodada válida (a 2ª
  falhou no clique). Servem de referência, não de distribuição.
- **Execução ponta a ponta do script commitado com `--click`**: não feita (janela de GUI combinada
  com o orquestrador). A tabela acima veio do protótipo que virou o script. O script commitado
  rodou **uma** rodada, sem `--click` e sem `--allow-unnotarized` (diálogo respondido por quem
  estava na máquina; o código começou 18,5 s depois do prompt), e passou nos caminhos de erro
  (`sh -n`, `--rounds` fora de 1..5, `--timeout` não numérico, app inexistente, flag sem valor,
  `--click` sem `Y`, bundle sem Developer ID recusado). O caminho `--click` + captura + falha em
  5 s não foi exercitado.

## Como reproduzir

```sh
# app já aberto? feche antes: o script se recusa a rodar com outra instância
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3
# com dispensa automática do diálogo (coordenadas do botão Abrir neste display: 908,438)
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3 --click 908,438
```
