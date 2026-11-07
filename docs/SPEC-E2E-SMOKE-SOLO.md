# SPEC — cada spec do smoke passa sozinho, e a falha carrega o log cru

Issue-mãe: [#251](https://github.com/Heldinhow/openridr/issues/251). Frente anterior: #215/#56
(smoke noturno com issue de rastreio).

## Problem Statement

O único caminho automatizado de Build (`tests/e2e/build.spec.ts`) falhou na run
[42406578543](https://github.com/Heldinhow/openridr/actions/runs/42406578543) quando foi o primeiro e
único spec da execução: o primeiro build do fixture terminou em `Build failed · exit 1` onde o teste
esperava `Build succeeded`. O mesmo spec passa quando roda depois de `workspace.spec.ts` — e, na mesma
run, o **segundo** teste do mesmo arquivo (mesmo fluxo: abrir, confiar, restaurar, construir) passou em
17,6 s.

Quem olha a run vermelha não consegue diagnosticar nada: a mensagem de erro carrega só o estado da UI
(`Build failed · exit 1`), o log cru do `dotnet build` fica no DOM e se perde, e a run não sobe
artefato. O resultado é o pior dos dois mundos: o smoke fica vermelho sem dizer por quê, e a única
cobertura automatizada de Build vira candidata a quarentena por "flake" não investigado.

## Solution

1. Toda espera de estado de build no spec passa a carregar, na falha, o **log cru do build** — mais o
   comando e o cwd que o produziram. A próxima run vermelha é diagnosticável sozinha, sem instrumentar
   de novo.
2. Com isso, determinar a causa numa runner fria de verdade (build a frio vs. corrida com o restore
   automático) e corrigi-la na origem: no spec, se for espera legítima; no host, se for defeito do host.
3. Fixar o contrato que faltava: **cada spec da lista do smoke passa sozinho, numa máquina fria**. A
   ordem de `scripts/e2e-smoke.txt` não pode ser o que esconde a falha, e `build.spec.ts` sozinho no
   runner é critério de aceite.

## User Stories

1. Como mantenedor, quero que a run vermelha do smoke traga o log cru do build na mensagem de falha,
   para diagnosticar sem reproduzir localmente.
2. Como mantenedor, quero saber se a falha de `build.spec.ts` é dependência de ordem ou primeiro build
   a frio, para corrigir a causa em vez de reordenar a lista.
3. Como mantenedor, quero que `build.spec.ts` passe sozinho numa runner nova, para que a cobertura de
   Build não dependa de outro spec ter rodado antes.
4. Como sessão de agente, quero provar o fix do jeito que a falha apareceu (spec sozinho, máquina fria),
   para não fechar a issue com uma prova que só funciona na ordem da lista.
5. Como sessão de agente, quero que a falha de build carregue comando, cwd e log, para não gastar uma
   rodada de runner só para descobrir o que o `dotnet` disse.
6. Como mantenedor, quero que a regra "spec do smoke passa sozinho" esteja escrita onde a lista mora,
   para que o próximo spec adicionado nasça com esse contrato.
7. Como mantenedor, quero que a tabela de falhas conhecidas (`docs/evidence/test-stability.md`) diga o
   que foi medido e o que ficou sem prova, para não tratar flake conhecido como passe.
8. Como sessão de agente, quero que a issue de rastreio aberta pela minha run de diagnóstico diga que é
   diagnóstico da #251 e em que branch, para não ser confundida com a noturna da `main`.
9. Como mantenedor, quero que a run verde de prova feche a issue de rastreio que a minha run vermelha
   abriu, sem fechar a issue que a noturna da `main` abriu no mesmo período.
10. Como mantenedor, quero que a correção não mude comportamento do produto, para que o Build continue
    com o mesmo contrato (restore explícito, `--no-restore`, exit e log crus).

## Implementation Decisions

- **O instrumento é local ao spec.** Um helper em `build.spec.ts` envolve as esperas de `build-state` e,
  na falha, anexa à mensagem o texto de `build-log`, o `build-command` e o `build-cwd`. Não vira
  utilitário compartilhado em `tests/e2e/`: um chamador só não justifica a abstração.
- **O diagnóstico é na runner, não no palpite.** `scripts/e2e-smoke.txt` reduzido a
  `tests/e2e/build.spec.ts` numa branch de trabalho, `workflow_dispatch` do `e2e-smoke.yml`, e o log cru
  que o instrumento agora carrega é a evidência. Runs locais quentes e com `NUGET_PACKAGES`/
  `DOTNET_CLI_HOME` frios já foram medidas e **não** reproduzem: cache NuGet frio está descartado como
  causa única.
- **A causa decide o fix.** Se o log nomear um defeito determinístico do host (por exemplo, corrida
  entre o restore automático e o build com `--no-restore`), o conserto é no host, com teste Rust que
  falha antes, dentro desta issue enquanto for pequeno; se virar mudança de contrato, vira issue filha.
  Se o log mostrar falha transiente de primeira execução a frio, o spec re-tenta o primeiro build uma
  vez e a falha final continua carregando o log cru.
- **A ordem da lista não é o fix.** `scripts/e2e-smoke.txt` mantém a ordem atual; o que muda é o
  contrato: cada spec listado passa sozinho. A regra é escrita no cabeçalho da lista e em
  `docs/evidence/test-stability.md`.
- **Nada de produto muda.** Nenhum `data-testid`, nenhuma dependência nova, nenhuma mudança em
  `src/**`, `crates/**` (salvo se o diagnóstico apontar defeito do host) ou no workflow noturno.
- **Issue de rastreio do smoke é efeito colateral aceito.** A run vermelha de diagnóstico abre (ou
  reescreve) a issue de rastreio pelo desenho da #215; o corpo é editado dizendo que é diagnóstico da
  #251 na branch X. Quando a run verde de prova fechar a issue, a sessão confere que ela ficou fechada;
  issue aberta pela noturna da `main` no mesmo período não é tocada.
- **Registro.** Decisão em `docs/decisions.md`; evidência em `docs/evidence/e2e-smoke-solo.md`; a
  entrada da falha em `docs/evidence/test-stability.md` sai de "não investigado" para o que foi medido.

## Testing Decisions

- **Bom teste aqui é comportamento externo**: o estado mostrado na UI (`build-state`) e o texto cru do
  log que o host produziu. O helper não muda nenhuma asserção existente; só enriquece a mensagem de
  falha, então o spec continua provando o mesmo contrato.
- **Seam 1 (o mais alto, já existe)**: o spec Playwright `tests/e2e/build.spec.ts`, rodando sozinho
  contra host real e fixture real.
- **Seam 2 (já existe)**: a lista `scripts/e2e-smoke.txt` + `workflow_dispatch` do `e2e-smoke.yml` —
  é a única forma de rodar numa runner fria de verdade.
- **Seam 3 (só se a causa for do host, já existe)**: teste Rust do host com `dotnet` 8.0.408 real
  (`crates/host/tests/host.rs`), com falha-antes medida.
- **Prova de falha-antes do instrumento**: apontar o helper para um build que falha (cópia do fixture
  com erro de compilação antes do primeiro build) e conferir que a mensagem carrega o log cru; sem o
  helper, a mensagem não tem o log. Local, barato, sem runner.
- **Prior art**: as asserções de `build.spec.ts` (estado, comando, cwd, sdk, log, problemas) e o
  `expect(actual, message)` do Playwright, já usado no repo quando a mensagem precisa de contexto.

## Out of Scope

- Reordenar `scripts/e2e-smoke.txt` para esconder a falha.
- Consertar os outros specs em quarentena da #67 (`solution-git-status.spec.ts:111` é a #245).
- Mudar o workflow noturno, o script de rastreio ou a cadência do smoke.
- Windows e macOS Intel continuam não verificados.
- Aumentar o conjunto de specs do smoke.

## Further Notes

- Medições desta sessão: `build.spec.ts` 2/2 verde quente (1,3 min) e 2/2 verde com `NUGET_PACKAGES` e
  `DOTNET_CLI_HOME` frios (1,1 min), no par de portas 1925/4817.
- Na run vermelha, o segundo teste do arquivo passou: o que falha é o **primeiro build da máquina**, não
  o fluxo do spec. Isso torna improvável a hipótese de "estado de workspace deixado por
  `workspace.spec.ts`" — cada teste copia o fixture e abre a própria cópia.
- `restore` e `build` usam o mesmo `sdk_root` (8.0.408 do `global.json` do fixture); o build é
  `dotnet build <csproj> -c Debug -nologo -v:minimal --no-restore` sobre o restore da solução.
