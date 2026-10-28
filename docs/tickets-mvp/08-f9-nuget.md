# 08 — F9: NuGet

**What to build:** tool window NuGet com pacotes instalados e desatualizados por projeto, atualização de um pacote por vez com plano de comando e confirmação, restore explícito depois, e recusa honesta de CPM.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] A tool window lista pacotes instalados e desatualizados por projeto da solução avaliada, agrupados por TFM quando houver mais de um.
- [ ] Cada linha desatualizada mostra versão atual e nova, com fonte consultada; o estado de rede/offline preserva o log e mostra o erro bruto.
- [ ] Atualizar um pacote mostra o comando exato e exige confirmação; nada roda antes do aceite.
- [ ] O update usa `dotnet add <csproj> package <id> --version <v> --no-restore`; depois oferece restore explícito e nunca restaura sozinho.
- [ ] Solução com CPM (`Directory.Packages.props`) lista normalmente e recusa update com mensagem explicando o limite do MVP.
- [ ] Busca e instalação de pacote novo ficam fora e não aparecem como botão morto.
- [ ] Falha, cancelamento e saída desconhecida aparecem com log e estado próprio; ausência de dados nunca vira "tudo atualizado".
- [ ] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam (rede opt-in, parsing offline); commit local `feat(F9): ...`.
