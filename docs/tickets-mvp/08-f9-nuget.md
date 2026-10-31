# 08 — F9: NuGet

**What to build:** tool window NuGet com pacotes instalados e desatualizados por projeto, atualização de um pacote por vez com plano de comando e confirmação, restore explícito depois, e recusa honesta de CPM.

**Blocked by:** 01

**Status:** done (implementação) — `bun run build`, testes de unidade/boundary do NuGet e `tests/e2e/nuget.spec.ts` (3/3, Chromium do sistema) verdes; `dotnet list`/`dotnet add` reais e a suíte Playwright completa **NÃO VERIFICADOS** neste ambiente (Linux arm64 sem .NET SDK/macOS), registrados em `docs/implementation-notes.md`.

- [x] A tool window lista pacotes instalados e desatualizados por projeto da solução avaliada, agrupados por TFM quando houver mais de um.
- [x] Cada linha desatualizada mostra versão atual e nova, com fonte consultada; o estado de rede/offline preserva o log e mostra o erro bruto.
- [x] Atualizar um pacote mostra o comando exato e exige confirmação; nada roda antes do aceite.
- [x] O update usa `dotnet add <csproj> package <id> --version <v> --no-restore`; depois oferece restore explícito e nunca restaura sozinho.
- [x] Solução com CPM (`Directory.Packages.props`) lista normalmente e recusa update com mensagem explicando o limite do MVP.
- [x] Busca e instalação de pacote novo ficam fora e não aparecem como botão morto.
- [x] Falha, cancelamento e saída desconhecida aparecem com log e estado próprio; ausência de dados nunca vira "tudo atualizado".
- [ ] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam (rede opt-in, parsing offline); commit local `feat(F9): ...`. — `bun run build` e os testes NuGet passam; o spec NuGet passou 3/3; a suíte Playwright completa não foi executada e dois testes unitários de `tools.rs` (mais os boundary que exigem SDK) dependem de `/usr/local/share/dotnet` e falham por ausência do SDK (pré-existentes, fora desta fatia).
