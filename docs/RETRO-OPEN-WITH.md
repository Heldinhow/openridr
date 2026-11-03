# Retro — Abrir do Finder e do terminal (2026-11-02)

Escopo: associações `.sln`/`.slnx`/`.csproj` e pasta no bundle, recepção de arquivo pelo Finder
(app fechado e aberto), `openridr <caminho>` e documentação. Spec em
[SPEC-OPEN-WITH](SPEC-OPEN-WITH.md), tickets em `docs/tickets-open-with/`, evidência em
[evidence/03-open-with](evidence/03-open-with.md). Quatro commits de implementação + spec/tickets
+ dois commits de correções da revisão.

## O que funcionou

- **Spec e decisões antes do código.** A entrada de 2026-11-02 em `decisions.md` fixou o recorte
  (um único `workspace.open`, sem segundo fluxo de trust, sem executar nada ao abrir) e evitou
  que o shell Tauri duplicasse validação de caminho.
- **Teste vermelho no host.** O E2E de `open-external.spec.ts` falhou pelos motivos certos antes
  do código (caminho relativo, extensão maiúscula, `.slnx`, `not_found`) e passou depois.
- **Reuso do fluxo existente.** A abertura externa entra por `workspace.open`, então trust,
  `recent.json`, candidatos ambíguos e recusa de caminho não ganharam uma segunda implementação.
- **Revisão em dois eixos.** O eixo Spec pegou um gap real: diretório ambíguo vindo do Finder não
  mostrava os candidatos. O eixo Standards pegou `.slnx` faltando em run configurations,
  `projectName` dos recentes e textos de UI, além do self-check do launcher depender de
  `OPENRIDR_APP` exportado.
- **Teste nativo sem tocar no app do usuário.** Copiar o bundle para um temporário, trocar
  `CFBundleIdentifier` e assinar ad-hoc permitiu provar `open -a`/`RunEvent::Opened` com
  `OPENRIDR_HOME` isolado enquanto o `/Applications/OpenRidr.app` do Helder continuava aberto.

## O que surpreendeu

- **`bun run check` não compila `src-tauri`.** O crate tem `[workspace]` próprio e fica fora do
  workspace raiz; `tsc --noEmit && cargo check --workspace` não vê `src-tauri/src/main.rs`. Os
  testes do shell só rodaram com `cargo test --manifest-path src-tauri/Cargo.toml`. O pre-commit
  também não cobre o shell.
- **A lista de extensões de workspace é um contrato espalhado.** `.slnx` precisou entrar em
  `workspace_file`, `load_workspace`, `is_evaluation_input`, `resolve_directory`,
  `run_configs::solution_param`, no diálogo nativo, no `projectName` dos recentes e em três
  textos. A revisão achou dois pontos que o ticket não listava.
- **`~/.dotnet` vs SDK do sistema.** O ambiente tem 8.0.408 em `~/.dotnet` e 8.0.416/10.0.100 em
  `/usr/local/share/dotnet`; o E2E só passa com o PATH/DOTNET_ROOT do AGENTS. Sem isso, os specs
  de SDK podem pular ou falhar por ambiente, não por produto.
- **O evento nativo chega antes da UI.** `RunEvent::Opened` pode ocorrer antes do webview montar;
  guardar o caminho em `PendingOpen` e consumir no boot (além do evento) foi o que tornou a
  abertura fria confiável.
- **Um app instalado com o mesmo bundle id sequestra o `open -a`.** O LaunchServices reusa a
  instância de `/Applications`, não o bundle de debug. Testar sem uma cópia com id próprio
  arriscava abrir a fixture no app real do Helder.

## Review do diff contra o spec

- **Corrigido:** candidatos de diretório ambíguo agora aparecem na tela inicial quando não há
  workspace e em botões `external-open-candidate-N` quando há workspace montado; buffer sujo
  volta a usar a área de mensagem existente; `.slnx` entrou em run configurations/recentes e
  textos; `strip_project_suffix` reconhece `.slnx`; self-check do launcher é commitado e
  independente de `OPENRIDR_APP` exportado; notas e evidência deixaram de se contradizer.
- **Decisões registradas:** um prompt de Trust pendente não é substituído por outra abertura
  externa; `attribute()` aceitar aspas simples também em `ProjectReference` é XML válido e ficou
  documentado; `is_evaluation_input` invalida `.slnx` de propósito.
- **Sem bloqueadores restantes.** O recorte de execução de `.slnx` continua explícito: abrir e
  navegar funcionam; restore/build dependem de um SDK que suporte o formato.

## Verificação real

- `bun run check`: verde.
- `cargo test --workspace --no-fail-fast`: 6 alvos verdes + `openridr-host --test host` com
  70 passed / 1 failed (`file_access_is_confined_to_the_workspace`, conhecida).
- `cargo test --manifest-path src-tauri/Cargo.toml`: 2/2.
- `bun run test:e2e` (Chrome, PATH/DOTNET_ROOT de `~/.dotnet`): **233 passed, 6 failed, 1
  skipped** em 17,6 min. Cinco falhas são as conhecidas (`find-usages`, 2 de `run-configs`, 2 de
  `tests.spec.ts`); a sexta, `ui-fold.spec.ts` "debug frames and locals are in the first fold
  while paused", falhou com `ENOTEMPTY` no cleanup do diretório temporário e passou em isolamento
  (`ui-fold.spec.ts` + `open-external.spec.ts`: 9 passed). É flakiness de cleanup, não regressão
  desta fatia.
- `sh scripts/openridr-selfcheck.sh`: `ok: openridr self-check passed`.
- Bundle de debug: `bun run tauri build --debug --bundles app`; `Info.plist` com as 4
  `CFBundleDocumentTypes` e 3 `UTExportedTypeDeclarations`; `lsregister -dump` reclamando as UTIs.
- `open -a` com cópia de id único e `OPENRIDR_HOME=.scratch/host-home`: `.sln` com app fechado,
  `.csproj` com app aberto e pasta resolveram para o workspace; `recent.json` isolado gravou os
  caminhos e o processo continuou único. `~/Library/Application Support/OpenRidr` não foi alvo.

## Dívida / não feito

- **Sem prova visual/E2E da UI nativa:** prompt de Trust, faixa `external-open-error`, botões de
  candidato, recusa por buffer sujo, duplo clique no Finder e arrasto real no Dock. A prova foi
  por leitura, host e LaunchServices; o Playwright não dirige o app Tauri no macOS.
- **Launcher não rodado contra o bundle real:** `open` não repassa `OPENRIDR_HOME` do shell, e
  rodar sem isolamento gravaria no app home real. O self-check prova o argv; o `open -a` foi
  provado separadamente com `--env`.
- **`.slnx` sem restore/build:** SDK 8.0.408 não suporta o formato; o erro do dotnet é preservado.
- **Windows, macOS Intel, notarização e assinatura de distribuição:** fora do escopo verificado.
- **Checklists dos tickets:** ficaram abertos os itens sem prova visual (UI do ticket 02); o
  restante foi marcado com a evidência correspondente.

## Melhorias de ambiente sugeridas

1. **Incluir o shell no check.** Adicionar `cargo check --manifest-path src-tauri/Cargo.toml`
   (e, se o custo for aceitável, `cargo test --manifest-path src-tauri/Cargo.toml`) ao
   `bun run check` ou a um `check:desktop` chamado pelo pre-commit. Hoje uma quebra em
   `src-tauri` passa pelo hook.
2. **Uma única allowlist de extensões.** Extrair no host um helper de extensão/`WorkspaceKind`
   usado por `workspace_file`, `load_workspace`, `is_evaluation_input`, `resolve_directory` e
   `run_configs`, com teste de tabela; na UI, um `projectName` testado para `.slnx`. A revisão
   encontrou dois pontos fora do ticket justamente por essa dispersão.
3. **Receita reproduzível de LaunchServices.** Manter em `docs/install-app.md` o procedimento de
   cópia com `CFBundleIdentifier` próprio + `open --env OPENRIDR_HOME=...` para testar o evento
   nativo sem tocar no app instalado. Sem isso, o próximo agente vai repetir a descoberta ou
   arriscar a instância do usuário.
4. **Exigir o log vermelho do teste no relatório do implementador.** O ticket 01 implementou o
   E2E depois do código; o subagente reconstruiu o vermelho com HEAD temporário, mas o processo
   deveria pedir o comando e a falha inicial no próprio retorno.
5. **Silenciar a ponte HTTP por padrão.** `crates/host-http` imprime `METHOD /call` a cada
   requisição; a suíte E2E gera um log de vários MB e esconde o progresso do Playwright. Um
   `OPENRIDR_HTTP_LOG=1` (ou log só de erro) tornaria o log da suíte legível sem mudar o
   contrato de dev.

