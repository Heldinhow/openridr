# 15: Usar PTY real e confirmar fechamento

**What to build:** Após trust, usar um único terminal PTY com shell disponível e cwd explícitos, entrada/output reais e Ctrl+C. Fechar shell ativo requer decisão consciente e evidência de término, não apenas retorno ao prompt.

**Blocked by:** 02 — Confiar no workspace e separar SDKs.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Descobrir shell realmente disponível sem assumir zsh/PowerShell; abrir um PTY apenas após trust validado no host, com shell e cwd visíveis. Chamada direta antes de trust não cria shell no workspace.
- [ ] Entrada, output terminal e Ctrl+C reais funcionam no Mac e Windows; permitir `dotnet test` manual como complemento ao runner/fallback, sem fingir execução estática nem instalar SDK/packages automaticamente.
- [ ] Fechar shell ativo oferece manter terminal ou encerrar shell e fechar; cancelar conserva sessão. Prompt ativo é shell rodando, não sessão encerrada.
- [ ] Mostrar saída/estado e código do shell quando conhecido, não atribuí-lo ao último comando. Falha de encerramento, remanescentes e exit unknown ficam explícitos; preservar log de término após fechamento.
- [ ] Revogar trust impede novas sessões imediatamente e aplica confirmação/lista de processos ativos; troca/fechamento do workspace protege edições e encerra ou relata árvore/processos, sem declarar isolamento enquanto decisão pendente.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com teclado/foco/nomes acessíveis, redimensionamento/rolagem sem esconder ações e privacidade do output.

## Mapeamento à SPEC

AC-25/26 (PTY, input/Ctrl+C, fechamento e shell exit), AC-31 (parcial: terminal complementar; bloqueio/mensagem MTP em 16), AC-01/07, AC-32/33. SPEC P0.6; um terminal, sem splits/abas extras.

## Validação

UI real em ambos SOs: identificar shell/cwd, digitar comando, produzir output, interromper processo filho via Ctrl+C e testar manter/encerrar shell. Boundary PTY/processos cobre falha/permissão, ausência de shell, resize, revogação direta, término incompleto e exit ausente. CI sem terminal desktop interativo não prova esta tela; registrar pendência por ambiente, sem aprovar gate release só pelo log de CLI.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
