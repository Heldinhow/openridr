# Evidência — espaço livre no `doctor.sh` (issue #259, ticket #267)

Data: 2026-11-07. Alvo: macOS ARM64, worktree em volume externo com `$TMPDIR` no volume interno.

## O que mudou

`doctor.sh` do skill `verify-openridr` ganhou uma linha de espaço livre em MiB no volume de
`${TMPDIR:-/tmp}` (onde vivem o app home do E2E e o diretório de capturas): `ok` ≥ 3072 MiB, `note`
entre 1024 e 3071, `FAIL` (exit 1) abaixo de 1024. O número medido e o diretório aparecem na linha nos
três casos. Medição sem número (`df` ausente ou saída estranha) vira `note`, nunca `ok`. O script
continua somente leitura.

## Comandos e resultados

| Comando | Resultado |
|---|---|
| `sh .agents/skills/verify-openridr/scripts/doctor.sh` | exit 0; linha nova abaixo |
| `sh tests/shell/doctor.test.sh` | `all cases passed`, exit 0 (4000 MiB → `ok`/0; 1500 → `note`/0; 900 → `FAIL`/1; `df` sem número → `note`/0; sentinela não apagada) |
| `DOCTOR_SHELL=dash dash tests/shell/doctor.test.sh` | `all cases passed`, exit 0 — o teste e o doctor sob `dash` |
| `DOCTOR_SCRIPT=<doctor.sh de 5779634> sh tests/shell/doctor.test.sh` | **4 failure(s)**, exit 1 — falha-antes: nenhuma linha `disk` e 900 MiB saindo 0 |
| `DOCTOR_SCRIPT=<doctor.sh com `df -k` em vez de `df -Pk`> sh tests/shell/doctor.test.sh` | **3 failure(s)** — o `df` falso recusa a chamada que o doctor não deve fazer, então trocar o volume ou as flags quebra os casos |
| `sh -n .agents/skills/verify-openridr/scripts/doctor.sh` | ok |
| `shellcheck -s sh` no script e no teste | só `SC2015` (info) pré-existentes nas linhas não tocadas; linhas novas e o teste sem achado |

Linha impressa no disco real (caminho do `$TMPDIR` trocado por `<tmp>`):

```text
ok    disk 27414 MiB free on <tmp> (floor 3072 MiB: up to 4 parallel sessions, ~200 MiB E2E home each + caches)
```

O resto da saída do doctor não mudou: bun, cargo, `node_modules`, SDK 8.0.408, Chrome, bundle
(`note`: alpha.9 instalado vs alpha.11 no `package.json`), agent-desktop (`ok` nas duas permissões).

## Prova das severidades

O teste monta um `df` falso primeiro no `PATH`, uma árvore de repositório mínima com `node_modules` e
stubs de `bun`/`cargo`/`dotnet`/`pgrep`/`lsof`/`agent-desktop`, então o exit code depende só do caso de
disco. Nenhum disco foi enchido e nada foi apagado: um arquivo sentinela no diretório medido sobrevive
aos quatro casos.

## Não verificado

- **Superfícies A/B/C do `verify-openridr`**: nenhuma interface, host ou app mudou — a mudança é um
  script de diagnóstico. Não houve Playwright, navegador dirigido nem app Tauri real nesta rodada.
- **Disco realmente cheio**: as severidades `note`/`FAIL` foram provadas com `df` falso, não enchendo o
  volume. Encher o disco interno para provar `ENOSPC` de verdade não é aceitável nesta máquina.
- **Windows e macOS Intel**: não verificados, como o resto do repo.
- **Volume do worktree** (externo) e **pastas `.tmp*` órfãs**: fora de escopo por decisão do
  orquestrador (issue própria se for o caso).
