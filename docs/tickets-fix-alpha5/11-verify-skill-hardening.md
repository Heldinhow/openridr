# 11: verify-openridr para de mentir e de colidir

**What to build:** Os scripts e docs da skill `verify-openridr` fazem o que dizem: `down` só reporta "stopped" com o processo morto, sessões não apagam o estado uma da outra, o doctor valida a cópia sob teste, a extração de acessibilidade não imprime JSON cru, as receitas avisam sobre `--headed`, e os docs descrevem os diálogos reais.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída; scripts e docs provados nesta rodada (web e app real com a cópia sob teste). Captura de acessibilidade não verificada (permissões negadas).

- [x] `desktop.sh down` confere comando e liveness do PID, espera a morte, remove o pid e falha com exit não-zero listando instâncias restantes.
- [x] `desktop.sh up` confere que há exatamente uma instância depois do launch.
- [x] Estado padrão por usuário/sessão, impresso no output, com override por env; rotação em vez de `rm -rf`.
- [x] `doctor.sh` aceita bundle por argumento/`OPENRIDR_APP`, valida a versão contra `package.json` e diz qual caminho validou.
- [x] Linha do agent-desktop imprime `granted`/`denied` limpo, sem fragmento de JSON; checa também screen recording.
- [x] Docs de receitas dizem que `type`/`press` exigem `--headed` após `focus-window`; helper não aplica `--headed` a clique sem necessidade.
- [x] `features/` corrige `debug-consent-dialog` (não existe), o "diálogo de troca" inexistente e o status antigo de PERM_DENIED.
- [x] `verify-openridr` aponta para a skill `test-app-agent-desktop`.
- [x] `doctor.sh`, `web.sh up/status/down` e `desktop.sh up/down` executados nesta rodada como prova.
