# SPEC — App de dev em um comando e abertura de workspace por argv (issue #91)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe: [#91](https://github.com/Heldinhow/openridr/issues/91), achada validando a #46 (ver [RETRO-DEBUG-NO-STOP](RETRO-DEBUG-NO-STOP.md)) e paga de novo na #48 (ver [RETRO-GIT-DIFF](RETRO-GIT-DIFF.md)).

## Problema

Numa worktree nova, `bun run tauri dev` não abre. O `tauri-build` valida `bundle.resources` do `tauri.conf.json` no compile e o build morre com `resource path 'resources/dotnet' doesn't exist`, porque `src-tauri/resources/{dotnet,csharp-ls,netcoredbg}` só existem depois de vendorizar (rede) ou de um build completo. Mesmo com os resources no lugar, o binário de dev não está dentro de um `.app`, então `resources_root()` (`<exe>/../Resources`) aponta para um diretório que não existe: o par embutido (csharp-ls + runtime .NET 10 privado + netcoredbg) nunca é encontrado, o setup falha e a única saída é exportar três `OPENRIDR_BUNDLED_*` à mão — informação que hoje só está no cabeçalho de `crates/host/src/bundled.rs`.

A segunda metade do problema é da verificação: abrir workspace no app real por `agent-desktop` só é confiável pelo argv. O campo do Welcome não recebe digitação (`delivered_unverified`, valor vazio) e o clique em "Open" com o campo vazio abre o seletor nativo, que deixa o app preso em "Opening…". Nada disso está escrito na skill de verificação.

Custo medido: ~20 min de tentativa e erro por sessão que precise validar UI no app real, em duas sessões seguidas.

## Solução

Um comando, `sh scripts/dev-app.sh [caminho]`, que resolve o par embutido (de um `OpenRidr.app` instalado ou do `src-tauri/resources` vendorizado), exporta os três overrides e sobe o app de dev com o workspace já em argv. O build passa a compilar sem resources no lugar, sem cópia de ~150 MB para `target/debug` e sem rede.

A skill `verify-openridr` passa a documentar o argv como o caminho confiável para abrir workspace no app real, e o seletor nativo como a armadilha que trava o app.

## User stories

1. Como pessoa que desenvolve o OpenRidr, quero subir o app de dev numa worktree recém-criada com um comando, para não gastar 20 minutos descobrindo os três `OPENRIDR_BUNDLED_*`.
2. Como pessoa que desenvolve o OpenRidr, quero que esse comando use o par embutido de um app já instalado, para não precisar de rede nem de um build completo antes de olhar a UI.
3. Como pessoa que já vendorizou o par no repositório, quero que o comando prefira esse par, para a prova descrever os pins do HEAD e não a versão do app instalado.
4. Como pessoa que desenvolve o OpenRidr, quero abrir o app de dev já no workspace que estou testando, para não depender do campo do Welcome.
5. Como pessoa que dirige o app real com `agent-desktop`, quero saber que o argv é o caminho confiável, para não travar o app no seletor nativo.
6. Como pessoa que dirige o app real, quero saber que o campo do Welcome não recebe digitação, para não perder tempo com `delivered_unverified`.
7. Como pessoa que mantém o OpenRidr, quero que o comando recuse subir quando já existe um OpenRidr aberto, para o argv não ser entregue ao app instalado e o dev nunca aparecer.
8. Como pessoa que mantém o OpenRidr, quero uma mensagem clara quando não há par disponível, para saber se rodo `bun run app:install` ou os scripts de vendor.
9. Como pessoa que mantém o OpenRidr, quero um self-check que prove os exports, o argv e o caminho de erro, para o script não apodrecer em silêncio.
10. Como pessoa que revisa um PR, quero que o `AGENTS.md` ganhe só a linha do comando, para a mudança de processo não entrar de carona.
11. Como pessoa que mantém a skill de verificação, quero a receita de argv junto da superfície C, para a próxima sessão não redescobrir.
12. Como pessoa que usa o app de dev, quero que nada mude no app distribuído, porque o par embutido e o fluxo de release ficam como estão.

## Implementation decisions

- **O build de dev não exige resources no lugar** (decisão do grill, Q1a). O script roda `tauri dev --config '{"bundle":{"resources":null}}'`: o `--config` vira `TAURI_CONFIG` e o `tauri-build` aplica merge patch (RFC 7386), onde `null` **remove** a chave — `{"resources":{}}` não serve, porque objeto vazio é no-op no merge. Medido: build de 6,2 s e app abrindo o `.sln` do argv. A alternativa de copiar/simlinkar o par para `src-tauri/resources` faria o `tauri-build` copiar ~150 MB para `target/debug` a cada mudança de resource e continuaria precisando dos overrides; o bundle de debug (contorno do RETRO-GIT-DIFF) perde o hot reload.
- **Os três overrides são o contrato do par em dev.** `OPENRIDR_BUNDLED_CSHARP_LS` aponta a pasta com `CSharpLanguageServer.dll` + `.deps.json`; `OPENRIDR_BUNDLED_DOTNET_ROOT` a raiz do runtime com `dotnet`, `host/fxr/10.x` e `shared/Microsoft.NETCore.App/10.x`; `OPENRIDR_BUNDLED_NETCOREDBG` a pasta no layout de `Contents/Resources/netcoredbg`. Nada no host muda: os overrides já existem e são só de debug.
- **Fonte do par, em ordem** (Q2): os três `OPENRIDR_BUNDLED_*` já exportados (todos os três ou nenhum; parcial é erro) > `src-tauri/resources` completo (vendorizado, casa com os pins do HEAD) > `$OPENRIDR_APP` > `/Applications/OpenRidr.app` > erro. O script **não** vendoriza nem baixa nada: a mensagem de erro diz o que falta e as duas saídas (`bun run app:install`, ou `scripts/vendor-*.sh`). Quando a fonte é um `.app`, imprime a versão do bundle, porque um app antigo descreve a prova errada.
- **O par resolvido é validado antes de subir**, com as mesmas condições que o host usa para chamar o par de completo (entry dll + deps.json; muxer + `host/fxr` 10.x + runtime 10.x; binário do adaptador). Um par incompleto falha com o caminho que falta, em vez de virar "Retry setup" depois de 40 s de app aberto.
- **Workspace opcional em argv** (Q3): `sh scripts/dev-app.sh [caminho]` valida existência e `.sln`/`.slnx`/`.csproj`/pasta com as mesmas mensagens do `scripts/openridr`, resolve o caminho absoluto e o passa por `tauri dev -- <caminho>`. Sem argumento, o app abre no Welcome.
- **`OPENRIDR_HOME` não é isolado** (Q4): o script respeita o que o chamador exportou e não inventa home. Isolar é trabalho do `desktop.sh` da skill de verificação, que já faz isso.
- **Uma instância por vez** (Q5): o script recusa subir se `pgrep -x openridr` achar algo, como o `desktop.sh` faz. Sem isso o plugin de instância única entrega o argv ao app instalado e o binário de dev sai na hora.
- **Pré-requisitos explícitos**: `bun` no PATH (o `beforeDevCommand` roda `bun run dev`) e `node_modules/.bin/tauri` presente. Faltando qualquer um, o script para antes de compilar, apontando `bun install` — em vez de deixar o `tauri` baixar o CLI da rede no meio do comando.
- **Sem dependência nova, sem mudança de UI, sem `data-testid`.** Nada no app distribuído muda: o `--config` é por invocação e só no dev.
- **Documentação com escopo mínimo no `AGENTS.md`** (Q7, condição do orquestrador): só a seção **Comandos**, com a linha do comando. A receita de argv e a armadilha do seletor nativo ficam na skill `verify-openridr` (superfície C) e no `test-app-agent-desktop` (falhas conhecidas), não em regra de processo.

## Testing decisions

- **Um teste bom aqui prova o argv e o ambiente que o app recebe**, não o texto do script: com um `tauri` falso no `node_modules` de uma raiz falsa, o log registra `dev --config {"bundle":{"resources":null}} -- <caminho absoluto>` e os três exports apontando o par resolvido.
- **Seam:** o próprio `scripts/dev-app.sh`, executado a partir de uma raiz falsa (o script deriva a raiz de `$0`). Nenhum seam novo de produção, nenhuma variável de teste no script.
- **Prior art:** `scripts/openridr-selfcheck.sh` — mesma forma: `PATH`/`mktemp -d`, um binário falso que só grava o que recebeu, `trap` para limpar, e uma sequência de casos com `fail`.
- **Casos:** (a) par vendorizado em `src-tauri/resources` → exports + argv com `--config`; (b) fonte `OPENRIDR_APP` → exports apontando o `.app`, com a versão impressa; (c) sem par nenhum → exit diferente de 0 e mensagem com as duas saídas; (d) caminho de workspace inexistente → exit 2; (e) sem `node_modules/.bin/tauri` → exit diferente de 0 pedindo `bun install`.
- **O guard de instância única não entra no self-check** (depende de processo real): é verificado à mão na evidência.
- **Self-check não entra no CI** (Q6): nenhum `scripts/*-selfcheck.sh` entra hoje, e mexer em workflow é `risk:high`. Roda local, documentado no `AGENTS.md` junto do comando.
- **Verificação no app real** é a prova final: com o slot do app liberado, `scripts/dev-app.sh <sln>` numa worktree limpa, `agent-desktop` mostrando o Trust e depois a árvore da solution. O que não rodar entra como "não verificado".

## Out of scope

- Vendorizar automaticamente (rede) dentro do script.
- Isolar `OPENRIDR_HOME`, trocar portas do Vite/host ou gerenciar o servidor de dev.
- Windows e macOS Intel.
- Mudar o `tauri.conf.json`, o `beforeBuildCommand`, o fluxo de release ou qualquer coisa do par embutido no app distribuído.
- Consertar o campo do Welcome ou o seletor nativo: a issue documenta o contorno, não muda a UI.
- Playwright: a verificação de UI continua sendo o app real via `agent-desktop`.

## Further notes

- Evidência em `docs/evidence/dev-app.md`; decisão em `docs/decisions.md`; linha em `docs/README.md`; retro em `docs/RETRO-DEV-APP.md`.
- O `scripts/openridr` (launcher de "abrir com") não muda: ele abre o app instalado, não o de dev.
- A armadilha do seletor nativo é a mesma que o `RETRO-DEBUG-NO-STOP` registrou em 2026-11-06; aqui ela deixa de ser conhecimento de sessão.
