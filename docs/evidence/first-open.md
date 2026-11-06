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
- Bundle: 164 MB, 18 Mach-O (14 dylibs do runtime .NET 10 + `dotnet`, `netcoredbg`,
  `libdbgshim.dylib`, `openridr`), 423 arquivos, 192 na árvore `Resources/dotnet`.
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
| **E0** sem quarentena, cópia em disco (2 rodadas, 1ª com cache frio) | 0,06 / 0,12 | — | — | 0,13 / 0,55 | 1,00 / — | — | — |
| **E1** quarentena `0083`, cópia em disco (3) | 0,19 / 0,24 / 0,28 | **0,40 / 0,42 / 0,53** | 1,04 / 1,05 / 1,21 | 1,27 / 1,28 / 1,35 | 2,01 / 2,42 / 2,63 | **0,14–0,24** | **0,97–1,42** |
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

## O que o log do sistema mostra (rodada E1, 13:15:28)

```
13:15:28.132  open -n .../OpenRidr.app            (t0)
13:15:29.33   syspolicyd  GK performScan ...      (+1,20)   ← início da avaliação
13:15:29.61   CoreServicesUIAgent  present code-evaluation prompt   (+1,48)
13:15:29.61   XProtect finished executable analysis / suppress first launch warning: true
              (o processo fica parado em _dyld_start; LS: "launched with launchInQuarantine == true,
               so not starting the application")
13:15:52.5    clique em Abrir                      (+24,4)   ← latência do operador, não do sistema
13:15:52.6    CoreServicesUIAgent  progressed to 2% … 100%
13:15:52.8    nosso código roda
```

O `sample` que registrou "15–18 s em `_dyld_start`" estava olhando justamente esse intervalo de
espera do diálogo — um processo suspenso por quarentena fica em `_dyld_start` até alguém
responder, por mais tempo que se queira.

## Conclusão

1. **O atraso não está no bundle.** O diálogo do Gatekeeper aparece em **0,40–0,53 s** com o app
   notarizado/grampeado no disco, e o nosso código roda **0,14–0,24 s** depois do clique. Remover
   o ticket grampeado (E4) não melhora: 0,32–0,38 s até o diálogo. Os 18 Mach-O e o layout em
   `Contents/Resources` não aparecem como custo mensurável antes da decisão.
2. **O que o usuário percebe como travamento é, na maior parte, o diálogo esperando clique.** É
   comportamento do macOS para qualquer app baixado: não é redutível por assinatura, layout ou
   quantidade de Mach-O.
3. **O caminho "abrir direto do DMG" é caro e é redutível por instrução, não por código:** clique
   → janela leva **5,1–6,6 s** porque o sistema lê 164 MB da imagem comprimida (o `ditto` do
   mesmo app leva 1,95 s). Arrastar para Aplicações e abrir de lá é o caminho de ~1,4 s.
4. **Regra de corte aplicada:** as variantes de bundle (runtime em `tar.gz` para 1 Mach-O, runtime
   em `Contents/Frameworks`, bundle sem runtime) só seriam medidas se `prompt` ou `click→code`
   passassem de ~2 s. Não passaram em nenhuma condição, então **não foram medidas**.
5. **Diálogo visível e focável.** Com Finder e com T3 Code na frente (janela maximizada no
   centro), o diálogo apareceu **na frente e centralizado** em todas as rodadas com captura; e
   quando ele está na tela o *Return* equivale a *Abrir* (app arrancou 0,25 s depois do Return).

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
- **E0 (sem quarentena) e E3c (imagem fria)**: 1 rodada válida cada (a 2ª de cada condição falhou
  por clique errado/timeout); os números servem de referência, não de distribuição.
- **Execução do script commitado**: a medição usou o protótipo; o script final foi validado em
  `sh -n` e nos caminhos de erro (sem `agent-desktop`, `--rounds` fora de 1..5, app inexistente),
  mas não foi rodado ponta a ponta depois de commitado (janela de GUI combinada com o
  orquestrador).

## Como reproduzir

```sh
# app já aberto? feche antes: o script se recusa a rodar com outra instância
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3
# com dispensa automática do diálogo (coordenadas do botão Abrir neste display: 908,438)
sh scripts/measure-first-open.sh --app /Applications/OpenRidr.app --rounds 3 --click 908,438
```
