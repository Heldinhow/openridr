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

| condição | exec | prompt | click | code | window | click→code | click→window |
|---|---|---|---|---|---|---|---|
| **E0** sem quarentena, cópia em disco (2 rodadas; a 1ª com cache de página frio) | 0,12 / 0,06 | — | — | 0,55 / 0,13 | — / 1,00 | — | — |
| **E1** quarentena `0083`, cópia em disco (3) | 0,19 / 0,24 / 0,28 | **0,40 / 0,42 / 0,53** | 1,04 / 1,05 / 1,21 | 1,27 / 1,28 / 1,35 | 2,01 / 2,42 / 2,63 | **0,14–0,23** | **0,97–1,42** |
| **E4** sem `Contents/CodeResources` (ticket online, 2) | 0,16 / 0,21 | 0,32 / 0,38 | 0,76 / 2,04 | 0,97 / 2,26 | 2,35 / 3,40 | 0,21 / 0,22 | 1,36 / 1,60 |
| **E3** cópia de dentro do DMG montado (2) | 0,14 / 0,21 | 0,26 / 0,35 | 0,67 / 0,90 | 0,88 / 1,08 | 1,22 / 1,47 | 0,18 / 0,21 | 0,54 / 0,58 |
| **E3m** abrir direto do DMG montado, 1ª vez | 0,19 | 1,38 | 1,78 | 6,21 | 6,90 | 4,43 | 5,12 |
| **E3m** 2ª/3ª vez no mesmo caminho (aprovação memorizada) | 0,07 / 0,09 | — | — | 0,19 / 0,22 | 0,63 / 1,21 | — | — |
| **E3c** abrir direto de imagem **fria** (DMG recém-copiado, mount novo) | 0,21 | 1,51 | 2,27 | 7,71 | 8,85 | 5,44 | 6,57 |

Referências auxiliares:

- `spctl --assess --type execute -vv` no app grampeado: **0,27 s**; sem `Contents/CodeResources`
  (consulta online): **0,43 s**. A avaliação de assinatura do bundle inteiro é sub-segundo.
- `spctl --assess --type open --context context:primary-signature` no DMG: 0,43 s; montar o DMG
  quarentenado não apresenta diálogo.
- `ditto` do app de dentro do DMG montado para o disco: **1,95 s** (é o custo do arrasto).

## O que o log do sistema mostra

Linhas reais da rodada E1-1 (`open` às 13:21:05.844), com a origem de cada timestamp:

```
+0,17  syspolicyd          GK performScan (team K6M85697D4)      ← avaliação começa
+0,18  CoreServicesUIAgent XProtect finished executable analysis ← malware/signature check pronto
+0,52  CoreServicesUIAgent present code-evaluation prompt        ← diálogo na tela (0,52 s no total)
+1,05  clique em Abrir (agent-desktop)                           ← decisão
+1,07  CoreServicesUIAgent progressed to 0% … 100%               ← quarentena processada
+1,28  nosso código roda (RSS > 20 MB)
+2,01  janela do OpenRidr visível
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
espera, não verificação de assinatura — a avaliação inteira antes do diálogo levou 0,52 s.

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
   disco, o Gatekeeper leva **0,52 s** até pôr o diálogo na tela (0,17 s até começar a avaliação,
   0,18 s até o XProtect terminar). Tudo o que o bundle poderia economizar está, portanto,
   **limitado a menos de meio segundo** — inclusive número de Mach-O e layout. Remover o ticket
   grampeado (E4) não muda a ordem de grandeza: 0,32–0,38 s.
2. **O que o usuário percebe como travamento é o diálogo esperando clique**, e não o app. É
   comportamento do macOS para qualquer app baixado: o processo fica suspenso (`_dyld_start`) até
   alguém responder. Medido: 0,14–0,23 s do clique até o nosso código e 0,97–1,42 s até a janela.
3. **O caminho "abrir direto do DMG" é caro e é redutível por instrução, não por código:** clique
   → janela leva **5,1–6,6 s** porque o sistema lê 164 MB da imagem comprimida (o `ditto` do
   mesmo app leva 1,95 s). Arrastar para Aplicações e abrir de lá é o caminho de ~1,4 s.
4. **Regra de corte aplicada apenas onde ela mede o bundle.** As variantes de bundle (runtime em
   `tar.gz` para 1 Mach-O, runtime em `Contents/Frameworks`, bundle sem runtime) testariam o custo
   de avaliação, então a condição é o app **no disco**: ali nenhum marco passou de 0,53 s até o
   diálogo e 0,24 s depois do clique, muito abaixo do limite de ~2 s combinado — por isso **não
   foram medidas**. Nas condições que abrem de dentro da imagem (E3m/E3c) `click→code` passou de
   2 s, mas o custo ali é leitura da imagem (`ditto` 1,95 s), não avaliação de assinatura: variante
   de bundle não mudaria esse número.
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
  com o orquestrador). A medição da tabela usou o protótipo que virou o script. O script commitado
  rodou uma rodada **sem** `--click` (diálogo respondido por alguém, código 18,5 s depois do
  prompt) e passou nos caminhos de erro (`sh -n`, `--rounds` fora de 1..5, `--timeout` não
  numérico, app inexistente, flag sem valor, `--click` sem `Y`).

## Como reproduzir

```sh
# app já aberto? feche antes: o script se recusa a rodar com outra instância
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3
# com dispensa automática do diálogo (coordenadas do botão Abrir neste display: 908,438)
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3 --click 908,438
```
