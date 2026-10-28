# 15: Usar PTY real e confirmar fechamento

**What to build:** Após trust, usar um único terminal PTY com shell disponível e cwd explícitos, entrada/output reais e Ctrl+C. Fechar shell ativo requer decisão consciente e evidência de término, não apenas retorno ao prompt.

**Blocked by:** 02 — Confiar no workspace e separar SDKs.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Descobrir shell realmente disponível sem assumir zsh/PowerShell; abrir um PTY apenas após trust validado no host, com shell e cwd visíveis. Chamada direta antes de trust não cria shell no workspace.
- [ ] Entrada, output terminal e Ctrl+C reais funcionam no Mac e Windows; permitir `dotnet test` manual como complemento ao runner/fallback, sem fingir execução estática nem instalar SDK/packages automaticamente.
- [x] Fechar shell ativo oferece manter terminal ou encerrar shell e fechar; cancelar conserva sessão. Prompt ativo é shell rodando, não sessão encerrada.
- [x] Mostrar saída/estado e código do shell quando conhecido, não atribuí-lo ao último comando. Falha de encerramento, remanescentes e exit unknown ficam explícitos; preservar log de término após fechamento.
- [x] Revogar trust impede novas sessões imediatamente e aplica confirmação/lista de processos ativos; troca/fechamento do workspace protege edições e encerra ou relata árvore/processos, sem declarar isolamento enquanto decisão pendente.
- [x] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com teclado/foco/nomes acessíveis, redimensionamento/rolagem sem esconder ações e privacidade do output.

## Mapeamento à SPEC

AC-25/26 (PTY, input/Ctrl+C, fechamento e shell exit), AC-31 (parcial: terminal complementar; bloqueio/mensagem MTP em 16), AC-01/07, AC-32/33. SPEC P0.6; um terminal, sem splits/abas extras.

## Validação

UI real em ambos SOs: identificar shell/cwd, digitar comando, produzir output, interromper processo filho via Ctrl+C e testar manter/encerrar shell. Boundary PTY/processos cobre falha/permissão, ausência de shell, resize, revogação direta, término incompleto e exit ausente. CI sem terminal desktop interativo não prova esta tela; registrar pendência por ambiente, sem aprovar gate release só pelo log de CLI.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.

- 2026-10-28: Implementado e evidenciado em macOS 27.2 arm64 com PTY real (`portable-pty 0.9`, host only), shell descoberto (`/bin/zsh`; `/bin/sh` nos testes Rust), cwd no root do workspace, input/output reais, Ctrl+C (`0x03`) interrompendo `sleep 30` em <1 s, `dotnet test` manual real (2 testes aprovados), buffer de 4.000 linhas/120k com `outputSeq`, close confirmado (“Keep terminal”/“Terminate shell and close”), `SIGTERM then SIGKILL` observado com `exit unknown`, revogação selada e troca de workspace derrubando/selando o shell. **Windows x64 não foi executado** (ConPTY/COMSPEC/`TerminateProcess` só existem no código), então a caixa 2 permanece aberta; sem emulador de terminal e sem varredura de jobs fora do grupo/foreground — ver limitações. Evidência: [15-terminal-pty.md](../evidence/15-terminal-pty.md).
