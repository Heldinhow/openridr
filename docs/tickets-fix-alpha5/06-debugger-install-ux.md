# 06: Debugger instalado com transparência

**What to build:** O clique em Debug continua baixando o netcoredbg sozinho (decisão do Helder), mas o usuário vêprogresso, tem retry em falha e encontra em Settings/Workspace tools a origem, a versão e o checksum verificado do adaptador. O painel deixa de dizer "blocked on this platform" quando o binário existe.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Download automático mostra progresso observado (bytes), sem inventar porcentagem.
- [ ] SHA-256 é verificado antes de executar o binário; divergência aborta com erro claro.
- [ ] Falha de rede mostra erro acionável com botão de tentar de novo.
- [ ] Instalação parcial nunca aparece como pronta.
- [ ] Settings/Workspace tools mostra origem, versão e checksum do adaptador instalado.
- [ ] O plano não diz "stays blocked on this platform" quando o binário já está instalado e a sessão pode rodar.
- [ ] "experimental · disabled" continua explicando que é flag de sessão, com motivo.
- [ ] O fluxo revisável do `Install analysis` (csharp-ls) não muda.
- [ ] Testes de host para checksum/estado; prova no app real de progresso e retry.
