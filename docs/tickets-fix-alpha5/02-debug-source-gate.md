# 02: Debug após mudança externa de fonte

**What to build:** Depois de apagar um `.cs` por fora e rebuildar com sucesso, o Debug inicia em vez de acusar "saved source changed after this build" citando um arquivo que já não existe.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída — prova no app Tauri real pendente (`verify-openridr`).

- [x] Um arquivo ausente antes e depois do build não conta como "alterado desde o build".
- [x] Um arquivo que existia no build e foi apagado depois continua disparando o rebuild implícito (proteção preservada).
- [x] Um arquivo modificado depois do build continua bloqueando como hoje.
- [x] Teste de host cobre o caso criar → build → apagar → rebuild → gate liberado.
- [ ] Prova no app real: build verde após delete externo → Start debugging inicia.
