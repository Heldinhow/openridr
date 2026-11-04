# 06: Debugger instalado com transparência

**What to build:** O clique em Debug continua baixando o netcoredbg sozinho (decisão do Helder), mas o usuário vêprogresso, tem retry em falha e encontra em Settings/Workspace tools a origem, a versão e o checksum verificado do adaptador. O painel deixa de dizer "blocked on this platform" quando o binário existe.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída — prova no app Tauri real pendente (`verify-openridr`).

- [x] Download automático mostra progresso observado (bytes), sem inventar porcentagem.
- [x] SHA-256 é verificado antes de executar o binário; divergência aborta com erro claro.
- [x] Falha de rede mostra erro acionável com botão de tentar de novo.
- [x] Instalação parcial nunca aparece como pronta.
- [x] Settings/Workspace tools mostra origem, versão e checksum do adaptador instalado.
- [x] O plano não diz "stays blocked on this platform" quando o binário já está instalado e a sessão pode rodar.
- [x] "experimental · disabled" continua explicando que é flag de sessão, com motivo.
- [x] O fluxo revisável do `Install analysis` (csharp-ls) não muda.
- [ ] Testes de host para checksum/estado; prova no app real de progresso e retry.
