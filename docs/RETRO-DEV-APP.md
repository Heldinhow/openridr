# RETRO — App de dev em um comando e abertura por argv (issue #91)

Sessão de 2026-11-07, worktree `chore/issue-91-dev-app-one-command`, macOS arm64. Spec: [SPEC-DEV-APP](SPEC-DEV-APP.md). Tickets: #172 (script + self-check), #173 (docs). Evidência: [evidence/dev-app.md](evidence/dev-app.md).

## O que se sustentou

- **Medir antes de perguntar mudou o grill.** As oito perguntas foram enviadas com o resultado já medido (`resource path 'resources/dotnet' doesn't exist`, `{}` é no-op e `null` remove, build de 6,2 s, app abrindo o `.sln` do argv). O orquestrador aprovou tudo na primeira rodada, sem nenhuma pergunta de volta — a alternativa seria uma rodada perguntando "dá para passar uma flag?" e outra perguntando "e o par?".
- **A flag certa era a que remove, não a que esvazia.** `TAURI_CONFIG` passa por merge patch (RFC 7386): `{"bundle":{"resources":{}}}` é no-op e o build continua morrendo; `{"bundle":{"resources":null}}` remove a chave. Uma tentativa perdida, mas barata: o build já estava quente (6 s por rodada), não 5 min.
- **O self-check no molde do `openridr-selfcheck.sh` provou o contrato sem app, sem rede e sem compilar.** Raiz falsa + `bun`/`pgrep`/`tauri` falsos no PATH; o script deriva tudo de `$0`, então não precisou de nenhum seam de produção.
- **A prova no app real fechou o critério da issue.** Numa worktree sem `src-tauri/resources`, um comando levou de `pair from /Applications/OpenRidr.app 0.3.0-alpha.9` até a árvore da solution, com `trusted.json` gravado e **sem** `tools/` no home — prova de que o par veio dos overrides e nada foi baixado.
- **O guard de instância foi provado no app real, não só no self-check.** Com o app vivo, o script saiu com `exit 1` e `an OpenRidr is already running; close it first.`

## O que quebrou / custou

- **Abri o app sem pedir o slot.** O `pgrep` estava vazio, então tratei o slot como livre — mas outra sessão tinha acabado de receber a liberação. O orquestrador parou a sessão e fixou a regra: pedir o slot antes de abrir, mesmo com `pgrep` vazio. Custo: uma rodada de espera e uma mensagem de correção.
- **O primeiro `--config` foi o inútil.** `{"bundle":{"resources":{}}}` parece "sem resources" e não é: objeto vazio não remove chave em merge patch. O sintoma foi idêntico ao original (`resource path ... doesn't exist`), o que faz parecer que a ideia toda não funciona.
- **Caminho de `mktemp` no macOS tem `//` e o `/var` é symlink.** O self-check comparava texto literal e falhou duas vezes (`/var` vs `/private/var`) antes de normalizar a raiz uma vez. O script resolve o workspace com `pwd -P`; a expectativa tem que resolver igual.
- **A revisão independente achou dois médios que o self-check não pegava.** Runtime validado só por prefixo `10.` (aceitava `10.invalid` e prerelease, que o host recusa) e caminhos relativos que quebravam depois do `cd "$repo"`. Ambos viraram casos de teste — e o caso de caminho relativo só existe porque a revisão apontou.

## Revisão independente (rodada 1) — dois médios e três baixos

- **Médio, corrigido: validação do runtime mais frouxa que a do host.** `is_stable_10` passou a exigir `10.<minor>[.<patch>…]` numérico e sem prerelease, como `Version::parse` + `major == 10 && prerelease.is_none()`; `10.0.14-preview` e `10.invalid` ganharam caso.
- **Médio, corrigido: caminho relativo virava outro caminho.** `absolute()` resolve os três diretórios e o `$OPENRIDR_APP` antes do `cd`; caso novo invoca de fora do repo com `OPENRIDR_APP` relativo.
- **Baixo, corrigido: o argv era provado por `$*`.** O `tauri` falso passou a gravar um argumento por linha, com `grep -Fqx` por argumento e um workspace com espaços.
- **Baixo, corrigido: env herdado contaminava o self-check.** `unset` dos três `OPENRIDR_BUNDLED_*` e de `OPENRIDR_APP` no topo.
- **Baixo, respondido por escrito: links do índice.** `docs/evidence/dev-app.md` nasceu no mesmo commit; este retro é o passo 7 do fluxo e entra antes do PR.

## Bateria final (uma vez, no fim)

- `bun run check` verde; `bun test tests/unit` **89 pass, 0 fail**; `cargo fmt --all --check` e `cargo fmt --manifest-path src-tauri/Cargo.toml --check` limpos; `cargo clippy --workspace --all-targets -- -D warnings` verde.
- `cargo test -p openridr-host`: **duas falhas, as duas já na tabela** de [evidence/test-stability.md](evidence/test-stability.md), com diff sem nenhuma linha de Rust:
  - `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build` — carga (load average 6,35 durante a rodada); isolado passa em **3,30 s**.
  - `tools_install_is_real_verified_and_reused` — rede: o download do `.nupkg` não cabe no prazo de 120 s (**120,30 s** medidos, `left: "failed"`, `right: "ready"`), o mesmo caso da #93.

## Lições

- **Merge patch não apaga com objeto vazio.** Quando a ferramenta diz "merge com este JSON", o valor que remove é `null`, e `{}` é o valor que não faz nada. Testar a intenção ("sem resources") contra o mecanismo ("remover a chave") custou uma rodada de 6 s — barato porque o build estava quente.
- **Script que dá `cd` tem que absolutizar tudo antes.** Qualquer caminho exportado ou passado adiante depois de um `cd` é uma armadilha; a revisão achou porque olhou a ordem das linhas, não o caso feliz.
- **Validar "10.x" copiando o prefixo não é validar como o host.** O host parseia a versão e recusa prerelease; o script aceitava qualquer coisa começando com `10.`. Quando o host já tem a regra, a cópia em shell tem que citar a função que ela imita.
- **Self-check com raiz falsa é o barato que paga.** Nenhum seam de produção, nenhuma espera de app, e ainda assim os exports, o argv, os exit codes e as mensagens ficam presos.
- **O slot do app é do orquestrador, não do `pgrep`.** `pgrep` vazio diz que ninguém está rodando agora, não que a vez é sua.

## Follow-ups

- **`scripts/*-selfcheck.sh` não roda em CI.** Hoje só `scripts/ci-pr-body-selfcheck.sh` roda (job `governance`). Um job para os self-checks de script seria útil, mas é `risk:high` (workflow) e fora do escopo da #91.
- **O par vendorizado não foi provado no app real.** O self-check cobre exports e argv; o app real foi provado com o par do `/Applications`. Vendorizar exige rede e não muda o que o app recebe.
- **Não verificado:** Windows e macOS Intel; Playwright (não é prova de sessão).
